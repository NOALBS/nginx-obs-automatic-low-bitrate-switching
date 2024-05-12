use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::{
    sync::{mpsc, Mutex},
    time,
};
use tokio_tungstenite::tungstenite::Message as TMessage;
use tracing::{info, warn};

use crate::{
    chat::{self, ChatPlatform, HandleMessage, InternalUpdate},
    config, error, ChatSender,
};

const KICK_CHAT_WS: &str = "wss://ws-us2.pusher.com/app/eb1d5f283081a78b932c?protocol=7&client=js&version=7.6.0&flash=false";

pub struct Kick {
    _req_client: reqwest::Client,
    chat: KickChat,
}

impl Kick {
    pub fn new(chat_handler_tx: ChatSender) -> Self {
        let client = reqwest::Client::new();
        let chat = KickChat::connect(chat_handler_tx);

        Self {
            _req_client: client,
            chat,
        }
    }

    pub async fn join_channel(&self, platform: config::ConfigChatPlatform, channel: String) {
        info!("Joining channel: {}", channel);

        let config::ConfigChatPlatform::Kick(config) = platform else {
            panic!("Join called with wrong platform");
        };

        let channel = if config.use_irlproxy.unwrap_or_default() {
            tracing::error!("IRL Proxy is not implemented yet");
            return;
        } else {
            let config::KickConfig {
                channel_id,
                chatroom_id,
                ..
            } = config;

            let (Some(channel_id), Some(chatroom_id)) = (channel_id, chatroom_id) else {
                tracing::error!("Kick channel_id or chatroom_id is not set, ignoring channel");
                return;
            };

            Channel {
                username: channel,
                channel_id,
                chatroom_id,
            }
        };

        self.chat.add_channel(channel).await;
    }
}

#[async_trait]
impl super::ChatLogic for Kick {
    async fn send_message(&self, channel: String, message: String) {
        tracing::debug!(?channel, ?message, "Sending message to KICK");
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct Channel {
    /// Kick username, which seems to be the slug
    pub username: String,

    /// Kick channel id
    pub channel_id: usize,

    /// Kick chatroom id
    pub chatroom_id: usize,
}

#[derive(Debug)]
enum InnerMessage {
    Subscribe(Channel),
}

struct KickChat {
    channels: Arc<Mutex<Vec<Channel>>>,
    inner_tx: mpsc::UnboundedSender<InnerMessage>,
    inner_handle: tokio::task::JoinHandle<()>,
}

impl Drop for KickChat {
    fn drop(&mut self) {
        self.inner_handle.abort();
    }
}

impl KickChat {
    fn connect(chat_handler_tx: mpsc::Sender<HandleMessage>) -> Self {
        let (inner_tx, inner_rx) = mpsc::unbounded_channel();
        let channels = Arc::new(Mutex::new(Vec::new()));

        let inner_channels = channels.clone();
        let inner_handle = tokio::spawn(async move {
            let mut inner = Inner::new(inner_rx, inner_channels, chat_handler_tx).await;
            inner.kick_conn_loop().await;
        });

        Self {
            channels,
            inner_tx,
            inner_handle,
        }
    }

    async fn add_channel(&self, channel: Channel) {
        let mut channels = self.channels.lock().await;

        if channels.contains(&channel) {
            return;
        }

        self.send_inner(InnerMessage::Subscribe(channel.to_owned()));
        channels.push(channel);
    }

    fn send_inner(&self, msg: InnerMessage) {
        self.inner_tx.send(msg).unwrap();
    }
}

struct Inner {
    inner_rx: mpsc::UnboundedReceiver<InnerMessage>,
    channels: Arc<Mutex<Vec<Channel>>>,
    chat_handler_tx: mpsc::Sender<HandleMessage>,
    connection: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    last_pong: time::Instant,
}

impl Inner {
    async fn new(
        inner_rx: mpsc::UnboundedReceiver<InnerMessage>,
        channels: Arc<Mutex<Vec<Channel>>>,
        chat_handler_tx: mpsc::Sender<HandleMessage>,
    ) -> Self {
        Self {
            inner_rx,
            channels,
            chat_handler_tx,
            connection: get_connection().await,
            last_pong: time::Instant::now(),
        }
    }

    async fn kick_conn_loop(&mut self) {
        let mut activity_timeout = time::interval(Duration::from_secs(120));
        let (ping_tx, mut ping_rx) = mpsc::unbounded_channel();

        loop {
            loop {
                tokio::select! {
                    msg = self.inner_rx.recv() => {
                        if let Some(msg) = msg {
                            match msg {
                                InnerMessage::Subscribe(ss) => {
                                    self.subscribe(&format!("channel.{}", ss.channel_id)).await;
                                    self.subscribe(&format!("chatrooms.{}.v2", ss.chatroom_id)).await;
                                }
                            }
                        }
                    }
                    msg = self.connection.select_next_some() => {
                        activity_timeout.reset();

                        match msg {
                            Ok(msg) => if let Err(e) = self.handle_message(msg).await {
                                tracing::debug!(?e, "Error");
                            },
                            Err(e) => {
                                tracing::error!(?e, "Error");
                                break;
                            }
                        }
                    }
                    _ = activity_timeout.tick() => {
                        self.ping().await;

                        // Check last pong time after 30 seconds
                        let p_tx = ping_tx.clone();
                        tokio::spawn(async move {
                            time::sleep(Duration::from_secs(30)).await;
                            p_tx.send(()).unwrap();
                        });
                    }
                    _ = ping_rx.recv() => {
                        if self.last_pong.elapsed() >= Duration::from_secs(30) {
                            tracing::error!("Timed out, reconnecting");
                            break;
                        }
                    }
                };
            }

            tracing::info!("Disconnected from KICK chat");
            self.connection = get_connection().await;
            self.reconnect_subscriptions().await;
        }
    }

    async fn handle_message(&mut self, msg: TMessage) -> Result<(), error::Error> {
        if let TMessage::Ping(_) = msg {
            self.last_pong = time::Instant::now();
        }

        let TMessage::Text(text) = msg else {
            return Ok(());
        };

        let event: Event = serde_json::from_str(&text)?;
        tracing::debug!(?event, "Received message");

        match event.data {
            EventData::ChatMessageEvent(msg) => {
                if msg.kind != "message" {
                    return Ok(());
                }

                let mut permission = chat::Permission::Public;

                if msg
                    .sender
                    .identity
                    .badges
                    .iter()
                    .any(|e| e.kind == "moderator")
                {
                    permission = chat::Permission::Mod;
                }

                if msg
                    .sender
                    .identity
                    .badges
                    .iter()
                    .any(|b| b.kind == "broadcaster")
                {
                    permission = chat::Permission::Admin;
                }

                let Some(channel) = self.chatroom_id_to_username(msg.chatroom_id).await else {
                    tracing::error!("Chatroom id not found for {}", msg.chatroom_id);
                    return Ok(());
                };

                self.chat_handler_tx
                    .send(HandleMessage::ChatMessage(chat::ChatMessage {
                        platform: ChatPlatform::Kick,
                        permission,
                        channel,
                        sender: msg.sender.slug,
                        message: msg.content,
                    }))
                    .await
                    .unwrap();
            }
            EventData::HostRaidEvent(event) => {
                tracing::debug!(?event, "Raided");

                let target = chat::RaidedInfo {
                    target: event.hosted.slug,
                    display: event.hosted.username,
                    platform: ChatPlatform::Kick,
                };

                self.chat_handler_tx
                    .send(chat::HandleMessage::InternalChatUpdate(
                        chat::InternalChatUpdate {
                            channel: event.channel.slug,
                            platform: ChatPlatform::Kick,
                            kind: InternalUpdate::Raided(target),
                        },
                    ))
                    .await
                    .unwrap();
            }
            EventData::Pong => {
                self.last_pong = time::Instant::now();
            }
            _ => {}
        }

        Ok(())
    }

    async fn send<'a>(&mut self, request: &Request<'a>) -> Result<(), error::Error> {
        let json = serde_json::to_string(request)?;

        if self.connection.send(TMessage::Text(json)).await.is_err() {
            tracing::error!("Error sending request to KICK");
        }

        Ok(())
    }

    async fn subscribe(&mut self, channel: &str) {
        let _ = self.send(&Request::Subscribe { auth: "", channel }).await;
    }

    async fn ping(&mut self) {
        let _ = self.send(&Request::Ping {}).await;
    }

    async fn chatroom_id_to_username(&self, id: usize) -> Option<String> {
        let users = self.channels.lock().await;
        users
            .iter()
            .find(|u| u.chatroom_id == id)
            .map(|u| u.username.to_owned())
    }

    async fn reconnect_subscriptions(&mut self) {
        let users = {
            let users_lock = self.channels.lock().await;
            users_lock
                .iter()
                .map(|user| (user.channel_id, user.chatroom_id))
                .collect::<Vec<_>>()
        };

        for (channel_id, chatroom_id) in users {
            self.subscribe(&format!("channel.{}", channel_id)).await;
            self.subscribe(&format!("chatrooms.{}.v2", chatroom_id))
                .await;
        }
    }
}

async fn get_connection(
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let mut retry_grow = 1;

    loop {
        info!("Connecting");

        if let Ok((ws_stream, _)) = tokio_tungstenite::connect_async(KICK_CHAT_WS).await {
            info!("Connected");
            break ws_stream;
        }

        let wait = 1 << retry_grow;
        warn!("Unable to connect");
        info!("trying to connect again in {} seconds", wait);
        time::sleep(Duration::from_secs(wait)).await;

        if retry_grow < 5 {
            retry_grow += 1;
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum Request<'a> {
    #[serde(rename = "pusher:subscribe")]
    Subscribe { auth: &'a str, channel: &'a str },
    #[serde(rename = "pusher:ping")]
    Ping {},
}

#[derive(Debug)]
pub struct Event {
    pub event: EventKind,
    pub data: EventData,
    pub channel: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    #[serde(rename = "pusher:pong")]
    Pong,
    #[serde(rename = "pusher:connection_established")]
    ConnectionEstablished,
    #[serde(rename = "pusher_internal:subscription_succeeded")]
    SubscriptionSucceeded,
    #[serde(rename = "App\\Events\\ChatMessageEvent")]
    ChatMessageEvent,
    #[serde(rename = "App\\Events\\ChatMoveToSupportedChannelEvent")]
    HostRaidEvent,
}

impl<'de> serde::Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct RawEvent {
            event: EventKind,
            data: String,
            channel: Option<String>,
        }

        let raw = RawEvent::deserialize(deserializer)?;
        let data = serde_json::from_str(&raw.data).map_err(serde::de::Error::custom)?;

        let data = match raw.event {
            EventKind::Pong => EventData::Pong,
            EventKind::ConnectionEstablished => EventData::ConnectionEstablished(
                serde_json::from_value(data).map_err(serde::de::Error::custom)?,
            ),
            EventKind::SubscriptionSucceeded => EventData::SubscriptionSucceeded(
                serde_json::from_value(data).map_err(serde::de::Error::custom)?,
            ),
            EventKind::ChatMessageEvent => EventData::ChatMessageEvent(
                serde_json::from_value(data).map_err(serde::de::Error::custom)?,
            ),
            EventKind::HostRaidEvent => EventData::HostRaidEvent(
                serde_json::from_value(data).map_err(serde::de::Error::custom)?,
            ),
        };

        Ok(Self {
            event: raw.event,
            channel: raw.channel,
            data,
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum EventData {
    Pong,
    ConnectionEstablished(ConnectionEstablished),
    SubscriptionSucceeded(SubscriptionSucceeded),
    ChatMessageEvent(ChatMessageEvent),
    HostRaidEvent(HostRaidEvent),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ConnectionEstablished {
    pub socket_id: String,
    pub activity_timeout: usize,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SubscriptionSucceeded {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HostRaidEvent {
    pub channel: HostRaidChannel,
    pub hosted: HostRaidHosted,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HostRaidChannel {
    pub slug: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HostRaidHosted {
    pub username: String,
    pub slug: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatMessageEvent {
    pub chatroom_id: usize,
    pub content: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub sender: ChatMessageSender,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatMessageSender {
    pub slug: String,
    pub identity: ChatMessageIdentity,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatMessageIdentity {
    badges: Vec<Badge>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Badge {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
    pub count: Option<usize>,
}
