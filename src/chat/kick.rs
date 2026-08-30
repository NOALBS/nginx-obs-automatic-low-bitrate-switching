use std::collections::HashMap;
use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::{
    sync::{Mutex, mpsc},
    time,
};
use tokio_tungstenite::tungstenite::Message as TMessage;
use tracing::{debug, error, info, warn};

use crate::{
    ChatSender,
    chat::{
        self, ChatPlatform, HandleMessage, InternalUpdate,
        kick_api::{self, KickApi, KickConfigStore, KickSender, KickState, ResolvedIds},
    },
    config, error,
    user_manager::UserManager,
};

const KICK_CHAT_WS: &str = "wss://ws-us2.pusher.com/app/32cbd69e4b950bf97679?protocol=7&client=js&version=7.6.0&flash=false";

pub struct Kick {
    req_client: reqwest::Client,
    chat: KickChat,
    /// `None` without an OAuth application: chat is then read-only.
    api: Option<Arc<KickApi>>,
    /// One sender per channel, keyed by the lowercased slug.
    senders: Mutex<HashMap<String, Arc<KickSender>>>,
    store: Arc<dyn KickConfigStore>,
}

impl Kick {
    pub fn new(chat_handler_tx: ChatSender, user_manager: UserManager) -> Self {
        let client = kick_api::http_client();
        let chat = KickChat::connect(chat_handler_tx);

        let api = kick_api::Credentials::from_env().map(|credentials| {
            info!("Kick: OAuth application configured, chat commands can be answered");
            Arc::new(KickApi::new(credentials))
        });

        if api.is_none() {
            info!(
                "Kick: KICK_CLIENT_ID and KICK_CLIENT_SECRET are not set, \
                 chat will be read-only"
            );
        }

        // The config file holds the token unless pointed somewhere writable.
        let store: Arc<dyn KickConfigStore> = match std::env::var(STATE_DIR_ENV) {
            Ok(dir) if !dir.trim().is_empty() => {
                info!("Kick: keeping the refresh token in {}", dir);
                Arc::new(StateDirStore {
                    dir: PathBuf::from(dir),
                })
            }
            _ => Arc::new(ConfigStore { user_manager }),
        };

        Self {
            req_client: client,
            chat,
            api,
            senders: Mutex::new(HashMap::new()),
            store,
        }
    }

    pub async fn join_channel(&self, platform: config::ConfigChatPlatform, channel: String) {
        info!("Joining channel: {}", channel);

        let config::ConfigChatPlatform::Kick(mut config) = platform else {
            panic!("Join called with wrong platform");
        };

        if config.use_irlproxy.unwrap_or_default() {
            error!("IRL Proxy is not implemented yet");
            return;
        }

        // What was remembered wins over the seed in the config file.
        let remembered = self.store.load(&channel).await;
        if remembered.refresh_token.is_some() {
            config.refresh_token = remembered.refresh_token;
        }
        config.channel_id = remembered.channel_id.or(config.channel_id);
        config.chatroom_id = remembered.chatroom_id.or(config.chatroom_id);
        config.broadcaster_user_id = remembered
            .broadcaster_user_id
            .or(config.broadcaster_user_id);

        self.resolve_missing_ids(&mut config, &channel).await;

        let (Some(channel_id), Some(chatroom_id)) = (config.channel_id, config.chatroom_id) else {
            error!(
                "Kick channel_id or chatroom_id is not set for {} and could not be \
                 looked up, ignoring channel",
                channel
            );
            return;
        };

        self.register_sender(&config, &channel).await;

        self.chat
            .add_channel(Channel {
                username: channel,
                channel_id,
                chatroom_id,
            })
            .await;
    }

    /// Fills in whatever the config left out, from the slug. Failing to look it
    /// up just keeps what the config had.
    async fn resolve_missing_ids(&self, config: &mut config::KickConfig, channel: &str) {
        let wants_broadcaster = matches!(config.send_as, Some(config::KickSendAs::User))
            && config.broadcaster_user_id.is_none();

        if config.channel_id.is_some() && config.chatroom_id.is_some() && !wants_broadcaster {
            return;
        }

        let Some((channel_id, chatroom_id, user_id)) =
            kick_api::lookup_channel(&self.req_client, channel).await
        else {
            warn!("Kick: could not look up the ids of {}", channel);
            return;
        };

        info!(
            "Kick: resolved {} to channel_id {}, chatroom_id {}",
            channel, channel_id, chatroom_id
        );

        config.channel_id.get_or_insert(channel_id);
        config.chatroom_id.get_or_insert(chatroom_id);
        config.broadcaster_user_id.get_or_insert(user_id);

        // A restart should not depend on that endpoint still being reachable.
        // Store what is in effect, not what was looked up: the config wins.
        if let (Some(channel_id), Some(chatroom_id), Some(broadcaster_user_id)) = (
            config.channel_id,
            config.chatroom_id,
            config.broadcaster_user_id,
        ) {
            self.store
                .store_ids(
                    channel,
                    ResolvedIds {
                        channel_id,
                        chatroom_id,
                        broadcaster_user_id,
                    },
                )
                .await;
        }
    }

    async fn register_sender(&self, config: &config::KickConfig, channel: &str) {
        let Some(api) = self.api.as_ref() else {
            return;
        };

        let Some(sender) = KickSender::new(api.clone(), config, self.store.clone()) else {
            info!(
                "Kick: no refreshToken configured for {}, commands will be read \
                 but not answered",
                channel
            );
            return;
        };

        let sender = Arc::new(sender);
        sender.announce_identity(channel).await;

        // With CONFIG_DIR nothing stops two profiles from naming one channel.
        if self
            .senders
            .lock()
            .await
            .insert(channel.to_owned(), sender)
            .is_some()
        {
            warn!(
                "Kick: more than one profile is configured for the channel {}, \
                 only one of them will answer its chat",
                channel
            );
        }
    }
}

#[async_trait]
impl super::ChatLogic for Kick {
    async fn send_message(&self, channel: String, message: String) {
        // The switcher builds its notifications with the raw config username,
        // while channels are joined lowercased.
        let channel = channel.to_lowercase();
        let sender = self.senders.lock().await.get(&channel).cloned();

        let Some(sender) = sender else {
            debug!(
                ?channel,
                ?message,
                "Kick: no chat:write credentials for this channel, dropping message"
            );
            return;
        };

        if let Err(e) = sender.send(&channel, &message).await {
            error!(?e, "Kick: could not send message to {}", channel);
        }
    }
}

/// Where to keep runtime state when the config file is read-only. The config
/// is then only the seed.
const STATE_DIR_ENV: &str = "NOALBS_STATE_DIR";

/// Keeps the state of each channel in its own file inside [`STATE_DIR_ENV`].
struct StateDirStore {
    dir: PathBuf,
}

impl StateDirStore {
    /// The slug is sanitised: a name like `../../etc/x` must not escape the dir.
    fn path_for(&self, channel: &str) -> PathBuf {
        let safe: String = channel
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect();

        self.dir.join(format!("kick-{safe}.json"))
    }

    async fn read(&self, channel: &str) -> KickState {
        let path = self.path_for(channel);

        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return KickState::default(),
            Err(e) => {
                warn!(?e, "Kick: could not read {}", path.display());
                return KickState::default();
            }
        };

        match serde_json::from_str(&contents) {
            Ok(state) => state,
            Err(e) => {
                // Starting from the seed beats refusing to start.
                error!(?e, "Kick: ignoring unreadable state in {}", path.display());
                KickState::default()
            }
        }
    }

    /// Reads, applies `edit` and writes the whole state back.
    async fn update<F>(&self, channel: &str, what: &str, edit: F)
    where
        F: FnOnce(&mut KickState) + Send,
    {
        let mut state = self.read(channel).await;
        edit(&mut state);

        if let Err(e) = self.write(channel, &state) {
            error!(
                ?e,
                "Kick: could not persist {} for {} in {}",
                what,
                channel,
                self.dir.display()
            );
        }
    }

    /// Through a temporary file: a crash halfway would leave no usable token.
    fn write(&self, channel: &str, state: &KickState) -> Result<(), error::Error> {
        fs::create_dir_all(&self.dir)?;

        let path = self.path_for(channel);
        let tmp = path.with_extension("tmp");

        // It holds a credential, so it is created private rather than made
        // private after the token is already on disk.
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);

        let mut file = options.open(&tmp)?;
        file.write_all(serde_json::to_string_pretty(state)?.as_bytes())?;
        file.sync_all()?;

        fs::rename(&tmp, &path)?;

        Ok(())
    }
}

#[async_trait]
impl KickConfigStore for StateDirStore {
    async fn load(&self, channel: &str) -> KickState {
        self.read(channel).await
    }

    async fn store_refresh_token(&self, channel: &str, refresh_token: &str) {
        self.update(channel, "the rotated refresh token", |state| {
            state.refresh_token = Some(refresh_token.to_owned());
        })
        .await;
    }

    async fn store_ids(&self, channel: &str, ids: ResolvedIds) {
        self.update(channel, "the resolved ids", |state| {
            state.channel_id = Some(ids.channel_id);
            state.chatroom_id = Some(ids.chatroom_id);
            state.broadcaster_user_id = Some(ids.broadcaster_user_id);
        })
        .await;
    }
}

/// Writes what NOALBS learned at runtime back into the user's config file.
struct ConfigStore {
    user_manager: UserManager,
}

enum Update {
    RefreshToken(String),
    Ids(ResolvedIds),
}

impl Update {
    fn apply(self, kick: &mut config::KickConfig) {
        match self {
            Self::RefreshToken(token) => kick.refresh_token = Some(token),
            Self::Ids(ids) => {
                kick.channel_id = Some(ids.channel_id);
                kick.chatroom_id = Some(ids.chatroom_id);
                kick.broadcaster_user_id = Some(ids.broadcaster_user_id);
            }
        }
    }

    fn what(&self) -> &'static str {
        match self {
            Self::RefreshToken(_) => "the rotated refresh token",
            Self::Ids(_) => "the resolved ids",
        }
    }
}

impl ConfigStore {
    /// Saves on a task of its own. Answering a command holds a read guard on
    /// the very state this has to write, and the lock is not reentrant, so
    /// writing here would deadlock the chat handler.
    fn save(&self, channel: String, update: Update) {
        let user_manager = self.user_manager.clone();

        tokio::spawn(async move {
            let what = update.what();
            let users = user_manager.get();
            let users = users.read().await;

            for user in users.values() {
                {
                    let mut state = user.state.write().await;

                    let Some(chat) = state.config.chat.as_mut() else {
                        continue;
                    };

                    // main() lowercases the slug before joining.
                    if !chat.username.eq_ignore_ascii_case(&channel) {
                        continue;
                    }

                    let config::ConfigChatPlatform::Kick(kick) = &mut chat.platform else {
                        continue;
                    };

                    update.apply(kick);
                }

                if let Err(e) = user.save_config().await {
                    error!(?e, "Kick: could not persist {} for {}", what, channel);
                }

                return;
            }

            warn!(
                "Kick: no user matched {}, {} will be lost on restart",
                channel, what
            );
        });
    }
}

#[async_trait]
impl KickConfigStore for ConfigStore {
    async fn store_refresh_token(&self, channel: &str, refresh_token: &str) {
        self.save(
            channel.to_owned(),
            Update::RefreshToken(refresh_token.to_owned()),
        );
    }

    async fn store_ids(&self, channel: &str, ids: ResolvedIds) {
        self.save(channel.to_owned(), Update::Ids(ids));
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
                    msg = self.connection.next() => {
                        let Some(msg) = msg else {
                            tracing::error!("WS stream got None, reconnecting");
                            break;
                        };

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

                let permission = msg.sender.identity.badges.iter().fold(
                    chat::Permission::Public,
                    |acc, badge| match badge.kind.as_str() {
                        "vip" => chat::Permission::Vip,
                        "moderator" => chat::Permission::Mod,
                        "broadcaster" => chat::Permission::Admin,
                        _ => acc,
                    },
                );

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

    async fn send(&mut self, request: &Request<'_>) -> Result<(), error::Error> {
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

async fn get_connection()
-> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
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

#[cfg(test)]
mod state_dir_tests {
    use super::*;

    fn store() -> (StateDirStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "noalbs-kick-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        (StateDirStore { dir: dir.clone() }, dir)
    }

    #[tokio::test]
    async fn an_empty_directory_yields_no_state() {
        let (store, _dir) = store();

        assert!(store.load("someone").await.refresh_token.is_none());
    }

    #[tokio::test]
    async fn the_token_survives_a_round_trip() {
        let (store, dir) = store();

        store.store_refresh_token("someone", "a-token").await;

        assert_eq!(
            store.load("someone").await.refresh_token.as_deref(),
            Some("a-token")
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn ids_and_token_live_together() {
        let (store, dir) = store();

        store.store_refresh_token("someone", "a-token").await;
        store
            .store_ids(
                "someone",
                ResolvedIds {
                    channel_id: 1,
                    chatroom_id: 2,
                    broadcaster_user_id: 3,
                },
            )
            .await;

        // Writing the ids must not drop the token, and the other way around.
        let state = store.load("someone").await;
        assert_eq!(state.refresh_token.as_deref(), Some("a-token"));
        assert_eq!(state.channel_id, Some(1));
        assert_eq!(state.chatroom_id, Some(2));
        assert_eq!(state.broadcaster_user_id, Some(3));

        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn a_corrupt_file_falls_back_to_the_seed() {
        let (store, dir) = store();
        fs::create_dir_all(&dir).unwrap();
        fs::write(store.path_for("someone"), "{not json").unwrap();

        // Refusing to start would be worse: the config still has a usable token.
        assert!(store.load("someone").await.refresh_token.is_none());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_channel_name_cannot_escape_the_directory() {
        let (store, dir) = store();

        let path = store.path_for("../../etc/passwd");

        assert_eq!(path.parent(), Some(dir.as_path()));
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("kick-______etc_passwd.json")
        );
    }
}
