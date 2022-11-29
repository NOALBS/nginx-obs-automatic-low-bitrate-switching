use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot, Mutex},
    time::{self, Instant},
};
use tokio_tungstenite::{
    tungstenite::{self, protocol::CloseFrame, Message as TMessage},
    MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, trace, warn};

use crate::chat;

pub type Writer = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, TMessage>;
pub type Reader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const TW_PUBSUB_WS: &str = "wss://pubsub-edge.twitch.tv";
const TW_PUBSUB_MAX_TOPICS: usize = 50;

#[derive(Error, Debug)]
pub enum TwitchPubsubError {
    #[error("websocket send error")]
    Send(#[source] tungstenite::Error),
    #[error("disconnected from twitch PubSub")]
    Disconnected,
    #[error("handle message error")]
    HandleMessageError,
    #[error("Receiver closed")]
    ReceiverClosed(#[from] tokio::sync::oneshot::error::RecvError),
}

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    #[serde(rename = "type")]
    kind: RequestKind,
    nonce: Option<String>,
    data: Option<RequestData>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
enum RequestKind {
    Listen,
    Ping,
    Unlisten,
}

#[derive(Serialize, Deserialize, Debug)]
struct RequestData {
    topics: Vec<String>,
    auth_token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    #[serde(rename = "type")]
    kind: MessageKind,
    data: Option<MessageData>,
    error: Option<String>,
    nonce: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MessageData {
    topic: String,
    #[serde(with = "json_string")]
    message: TopicMessage,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
enum MessageKind {
    Message,
    Pong,
    Response,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TopicMessage {
    RaidUpdateV2 { raid: RaidUpdateV2 },
    RaidCancelV2 { raid: RaidUpdateV2 },
    RaidGoV2 { raid: RaidUpdateV2 },
}

#[derive(Serialize, Deserialize, Debug)]
struct RaidUpdateV2 {
    creator_id: String,
    force_raid_now_seconds: u32,
    id: String,
    source_id: String,
    target_display_name: String,
    target_id: String,
    target_login: String,
    target_profile_image: String,
    transition_jitter_seconds: u32,
    viewer_count: u32,
}

mod json_string {
    use serde::de::{self, Deserialize, DeserializeOwned, Deserializer};
    use serde::ser::{self, Serialize, Serializer};
    use serde_json;

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        let j = serde_json::to_string(value).map_err(ser::Error::custom)?;
        j.serialize(serializer)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: DeserializeOwned,
        D: Deserializer<'de>,
    {
        let j = String::deserialize(deserializer)?;
        serde_json::from_str(&j).map_err(de::Error::custom)
    }
}

#[derive(Debug)]
struct InnerMessage {
    pub respond: oneshot::Sender<Result<(), TwitchPubsubError>>,
    pub message: String,
}

pub struct TwitchPubSub {
    pub ps_handle: tokio::task::JoinHandle<()>,
    write: mpsc::UnboundedSender<InnerMessage>,
    state: Arc<Mutex<State>>,
}

struct State {
    users: HashMap<String, User>,
    connected: bool,
}

struct User {
    username: String,
    last_raid: Instant,
}

impl TwitchPubSub {
    pub async fn connect(chat_handler_tx: mpsc::Sender<chat::HandleMessage>) -> Self {
        let (inner_tx, inner_rx) = mpsc::unbounded_channel();
        let state = Arc::new(Mutex::new(State {
            users: HashMap::new(),
            connected: false,
        }));
        let run_handle = tokio::spawn(run_loop(inner_rx, state.clone(), chat_handler_tx));

        TwitchPubSub {
            ps_handle: run_handle,
            write: inner_tx,
            state,
        }
    }

    pub async fn has_id(&self, twitch_id: &str) -> bool {
        let state = self.state.lock().await;
        state.users.contains_key(twitch_id)
    }

    pub async fn is_full(&self) -> bool {
        let state = self.state.lock().await;
        state.users.len() == TW_PUBSUB_MAX_TOPICS
    }

    async fn send(&self, request: Request) -> Result<(), TwitchPubsubError> {
        let message = serde_json::to_string(&request).unwrap();
        let (tx, rx) = oneshot::channel();
        let inner = InnerMessage {
            respond: tx,
            message,
        };

        self.write.send(inner).unwrap();
        rx.await.map_err(TwitchPubsubError::ReceiverClosed)?
    }

    async fn listen(&self, topics: Vec<String>) -> Result<(), TwitchPubsubError> {
        let listen_request = Request {
            kind: RequestKind::Listen,
            nonce: None,
            data: Some(RequestData {
                topics,
                auth_token: None,
            }),
        };

        self.send(listen_request).await
    }

    pub async fn add_raid(&self, twitch_id: String, username: String) {
        let mut state = self.state.lock().await;

        if state.connected {
            let topic = format!("raid.{}", twitch_id);
            if let Err(e) = self.listen(vec![topic]).await {
                error!(?e);
            }
        }

        state.users.insert(
            twitch_id,
            User {
                username,
                last_raid: Instant::now(),
            },
        );
    }
}

fn create_raid_topic_from_users(users: &HashMap<String, User>) -> Vec<String> {
    let mut topics = Vec::new();

    for id in users.keys() {
        topics.push(format!("raid.{}", id));
    }

    topics
}

async fn run_loop(
    inner_rx: mpsc::UnboundedReceiver<InnerMessage>,
    state: Arc<Mutex<State>>,
    chat_handler_tx: mpsc::Sender<chat::HandleMessage>,
) {
    // Spawn thread to handle inner requests
    let request_write = Arc::new(Mutex::new(None));
    tokio::spawn(handle_requests(inner_rx, request_write.clone()));

    loop {
        let ws_stream = get_connection().await;
        let (mut write, read) = ws_stream.split();

        {
            debug!("Sending listen");
            let mut lock = state.lock().await;
            let topics = create_raid_topic_from_users(&lock.users);

            let listen_request = Request {
                kind: RequestKind::Listen,
                nonce: None,
                data: Some(RequestData {
                    topics,
                    auth_token: None,
                }),
            };
            let listen_request = serde_json::to_string(&listen_request).unwrap();

            if let Err(e) = write.send(TMessage::Text(listen_request)).await {
                error!(?e, "error sending initial listen request");
                continue;
            };

            lock.connected = true;
        }

        {
            *request_write.lock().await = Some(write);
        }

        // Spawn thread to handle keepalive
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(keepalive(request_write.clone(), cancel_rx));

        // Handle messages
        if let Err(TwitchPubsubError::HandleMessageError) =
            handle_messages(read, state.clone(), chat_handler_tx.clone()).await
        {
            break;
        };

        // Disconnected
        let _ = cancel_tx.send(());

        {
            *request_write.lock().await = None;
        }

        {
            state.lock().await.connected = false;
        }
    }
}

async fn get_connection() -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    let mut retry_grow = 1;

    loop {
        info!("Connecting");

        if let Ok((ws_stream, _)) = tokio_tungstenite::connect_async(TW_PUBSUB_WS).await {
            info!("Connected");
            break ws_stream;
        }

        let wait = 1 << retry_grow;
        warn!("Unable to connect");
        info!("trying to connect again in {} seconds", wait);
        tokio::time::sleep(Duration::from_secs(wait)).await;

        if retry_grow < 5 {
            retry_grow += 1;
        }
    }
}

async fn keepalive(write: Arc<Mutex<Option<Writer>>>, mut cancel_rx: oneshot::Receiver<()>) {
    loop {
        if cancel_rx.try_recv().is_ok() {
            debug!("keepalive cancel received");
            break;
        }

        trace!("Sending ping");

        if let Some(w) = write.lock().await.as_mut() {
            if (w
                .send(TMessage::Text(
                    serde_json::to_string(&Request {
                        kind: RequestKind::Ping,
                        nonce: None,
                        data: None,
                    })
                    .unwrap(),
                ))
                .await)
                .is_err()
            {
                break;
            }
        }

        time::sleep(Duration::from_secs(290)).await;
    }

    debug!("Keepalive stopped")
}

async fn handle_messages(
    mut read: Reader,
    state: Arc<Mutex<State>>,
    chat_handler_tx: mpsc::Sender<chat::HandleMessage>,
) -> Result<(), TwitchPubsubError> {
    debug!("reading handle_message");
    while let Some(Ok(message)) = read.next().await {
        if let TMessage::Close(info) = &message {
            if let Some(CloseFrame { reason, .. }) = info {
                info!(%reason, "connection closed with reason");
            }

            continue;
        }

        if let TMessage::Text(text) = &message {
            let msg: Message = match serde_json::from_str(text) {
                Ok(o) => o,
                Err(e) => {
                    error!(?e, text, "failed to deserialize");
                    continue;
                }
            };

            trace!(?msg, "Received message");

            if let MessageKind::Message = msg.kind {
                if let Some(data) = msg.data {
                    if let TopicMessage::RaidGoV2 { raid } = data.message {
                        debug!(?raid, "Raided");

                        let (channel, ignore) = {
                            let mut lock = state.lock().await;
                            let user = lock.users.get_mut(&raid.source_id).unwrap();
                            let ignore = user.last_raid.elapsed().as_secs() < 10;
                            let channel = user.username.to_owned();

                            user.last_raid = Instant::now();

                            (channel, ignore)
                        };

                        if ignore {
                            continue;
                        }

                        chat_handler_tx
                            .send(chat::HandleMessage::InternalChatUpdate(
                                chat::InternalChatUpdate {
                                    channel,
                                    platform: chat::ChatPlatform::Twitch,
                                    kind: chat::InternalUpdate::StartedHosting,
                                },
                            ))
                            .await
                            .unwrap();
                    }
                }
            }
        }
    }

    warn!("Disconnected from twitch pubsub");

    Ok(())
}

async fn handle_requests(
    mut inner_rx: mpsc::UnboundedReceiver<InnerMessage>,
    write: Arc<Mutex<Option<Writer>>>,
) {
    while let Some(request) = inner_rx.recv().await {
        trace!(?request.message, "sending");

        let mut lock = write.lock().await;
        if let Some(w) = lock.as_mut() {
            let res = w
                .send(TMessage::Text(request.message))
                .await
                .map_err(TwitchPubsubError::Send);

            request.respond.send(res).unwrap();
        } else {
            request
                .respond
                .send(Err(TwitchPubsubError::Disconnected))
                .unwrap();
        }
    }
}

pub struct PubsubManager {
    clients: Arc<Mutex<Vec<TwitchPubSub>>>,
    chat_handler_tx: mpsc::Sender<chat::HandleMessage>,
}

impl PubsubManager {
    pub fn new(chat_handler_tx: mpsc::Sender<chat::HandleMessage>) -> Self {
        PubsubManager {
            clients: Arc::new(Mutex::new(Vec::new())),
            chat_handler_tx,
        }
    }

    pub async fn add_raid(&self, twitch_id: String, username: String) {
        let clients = &mut self.clients.lock().await;

        let already_listening = {
            let mut found = false;
            for client in clients.iter() {
                if client.has_id(&twitch_id).await {
                    found = true;
                    break;
                };
            }

            found
        };

        if already_listening {
            return;
        }

        let client = {
            let mut c = None;

            for client in clients.iter() {
                if !client.is_full().await {
                    c = Some(client);
                    break;
                }
            }

            if c.is_none() {
                let tps = TwitchPubSub::connect(self.chat_handler_tx.clone()).await;
                clients.push(tps);
                c = clients.last();
            }

            c.unwrap()
        };

        client.add_raid(twitch_id, username).await;
    }
}
