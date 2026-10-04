use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::Mutex,
    time::{Instant, timeout},
};
use tracing::{debug, error, info, warn};

use crate::{config, error};

pub(super) const AUTHORIZE_URL: &str = "https://id.kick.com/oauth/authorize";
pub(super) const TOKEN_URL: &str = "https://id.kick.com/oauth/token";
const CHAT_URL: &str = "https://api.kick.com/public/v1/chat";
const USERS_URL: &str = "https://api.kick.com/public/v1/users";

/// Kick rejects anything longer with a 400.
const MAX_MESSAGE_CHARS: usize = 500;

/// Access tokens last two hours; renew early so a message never races the expiry.
const REFRESH_MARGIN: Duration = Duration::from_secs(120);

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Cloudflare answers 403 (error 1010) to the default user agent of most clients.
const USER_AGENT: &str = concat!("noalbs/", env!("CARGO_PKG_VERSION"));

/// How the message is attributed in chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendAs {
    /// Posts with the bot badge in the channel of the token's owner.
    Bot,
    /// Posts as the token's owner, into `broadcaster_user_id`'s channel.
    User,
}

/// Ids of a channel, as looked up from its slug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedIds {
    pub channel_id: usize,
    pub chatroom_id: usize,
    pub broadcaster_user_id: u64,
}

/// What NOALBS learned at runtime for a channel, if anything.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct KickState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatroom_id: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broadcaster_user_id: Option<u64>,
}

/// Keeps what NOALBS learns at runtime so a restart does not have to learn it again.
#[async_trait]
pub trait KickConfigStore: Send + Sync {
    /// Takes precedence over the config file, which is only the seed.
    async fn load(&self, _channel: &str) -> KickState {
        KickState::default()
    }

    async fn store_refresh_token(&self, channel: &str, refresh_token: &str);
    async fn store_ids(&self, channel: &str, ids: ResolvedIds);
}

#[derive(Debug, Clone)]
pub struct Credentials {
    pub client_id: String,
    pub client_secret: String,
}

impl Credentials {
    /// Reads `KICK_CLIENT_ID` and `KICK_CLIENT_SECRET`, if both are set.
    pub fn from_env() -> Option<Self> {
        let client_id = std::env::var("KICK_CLIENT_ID").ok()?;
        let client_secret = std::env::var("KICK_CLIENT_SECRET").ok()?;

        if client_id.is_empty() || client_secret.is_empty() {
            return None;
        }

        Some(Self {
            client_id,
            client_secret,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    expires_in: u64,
    /// Only what Kick actually granted, which is not always what was asked for.
    #[serde(default)]
    pub scope: String,
}

/// Used by `noalbs kick-auth`; the bot itself only ever refreshes.
pub(super) async fn exchange_code(
    credentials: &Credentials,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, error::Error> {
    let api = KickApi::new(credentials.clone());

    let response = api
        .client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &credentials.client_id),
            ("client_secret", &credentials.client_secret),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(error::Error::KickApi(format!(
            "exchanging the code failed with {status}: {body}"
        )));
    }

    Ok(response.json().await?)
}

/// The account a token belongs to.
pub(super) async fn fetch_user(
    credentials: &Credentials,
    access_token: &str,
) -> Result<KickUser, error::Error> {
    KickApi::new(credentials.clone())
        .current_user(access_token)
        .await
}

#[derive(Debug, Deserialize)]
struct UsersResponse {
    data: Vec<KickUser>,
}

#[derive(Debug, Deserialize)]
pub struct KickUser {
    pub user_id: u64,
    pub name: String,
}

#[derive(Serialize)]
struct ChatMessageRequest<'a> {
    content: &'a str,
    #[serde(rename = "type")]
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    broadcaster_user_id: Option<u64>,
}

struct Token {
    access: String,
    expires_at: Instant,
}

/// Shared HTTP client plus the OAuth application's credentials.
pub struct KickApi {
    client: reqwest::Client,
    credentials: Credentials,
}

/// Every Kick host is behind Cloudflare, so nothing may use a default client.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("failed to build the Kick HTTP client")
}

impl KickApi {
    pub fn new(credentials: Credentials) -> Self {
        Self {
            client: http_client(),
            credentials,
        }
    }

    async fn refresh(&self, refresh_token: &str) -> Result<TokenResponse, error::Error> {
        let response = self
            .client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", &self.credentials.client_id),
                ("client_secret", &self.credentials.client_secret),
            ])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(error::Error::KickApi(format!(
                "refreshing the token failed with {status}: {body}"
            )));
        }

        Ok(response.json().await?)
    }

    async fn current_user(&self, access_token: &str) -> Result<KickUser, error::Error> {
        let response = self
            .client
            .get(USERS_URL)
            .bearer_auth(access_token)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(error::Error::KickApi(format!(
                "looking up the token's user failed with {status}: {body}"
            )));
        }

        response
            .json::<UsersResponse>()
            .await?
            .data
            .into_iter()
            .next()
            .ok_or_else(|| error::Error::KickApi("Kick returned no user for this token".into()))
    }
}

/// Sends messages to a single Kick channel, keeping its access token fresh.
pub struct KickSender {
    api: Arc<KickApi>,
    token: Mutex<Option<Token>>,
    /// Kept so a rotated refresh token can be persisted and reused on restart.
    refresh_token: Mutex<String>,
    send_as: SendAs,
    broadcaster_user_id: Mutex<Option<u64>>,
    store: Arc<dyn KickConfigStore>,
    /// Held across a renewal so only one is ever in flight.
    renewing: Mutex<()>,
}

impl KickSender {
    pub fn new(
        api: Arc<KickApi>,
        kick_config: &config::KickConfig,
        store: Arc<dyn KickConfigStore>,
    ) -> Option<Self> {
        let refresh_token = kick_config.refresh_token.clone()?;

        Some(Self {
            api,
            token: Mutex::new(None),
            refresh_token: Mutex::new(refresh_token),
            send_as: match kick_config.send_as {
                Some(config::KickSendAs::User) => SendAs::User,
                _ => SendAs::Bot,
            },
            broadcaster_user_id: Mutex::new(kick_config.broadcaster_user_id),
            store,
            renewing: Mutex::new(()),
        })
    }

    /// Not fatal on failure: the token is refreshed again on the first message.
    pub async fn announce_identity(&self, channel: &str) {
        match self.access_token(channel).await {
            Ok(token) => match self.api.current_user(&token).await {
                Ok(user) => info!(
                    "Kick: sending in {} as {} (user_id {}, mode {:?})",
                    channel, user.name, user.user_id, self.send_as
                ),
                Err(e) => warn!(?e, "Kick: could not read the token's identity"),
            },
            Err(e) => error!(?e, "Kick: unusable token for {}", channel),
        }
    }

    /// Returns a valid access token, refreshing it when it is about to expire.
    async fn access_token(&self, channel: &str) -> Result<String, error::Error> {
        if let Some(access) = self.fresh_token().await {
            return Ok(access);
        }

        // Kick rotates the refresh token on every use, so two renewals racing
        // would spend the same one twice.
        let _renewing = self.renewing.lock().await;

        if let Some(access) = self.fresh_token().await {
            return Ok(access);
        }

        self.renew_locked(channel).await
    }

    async fn fresh_token(&self) -> Option<String> {
        let token = self.token.lock().await;
        let token = token.as_ref()?;

        (token.expires_at > Instant::now() + REFRESH_MARGIN).then(|| token.access.clone())
    }

    /// Renews even if the current token still looks valid.
    async fn renew(&self, channel: &str) -> Result<String, error::Error> {
        let _renewing = self.renewing.lock().await;
        self.renew_locked(channel).await
    }

    async fn renew_locked(&self, channel: &str) -> Result<String, error::Error> {
        let current = self.refresh_token.lock().await.clone();
        let response = self.api.refresh(&current).await?;

        // Kick rotates it on every renewal, and losing the new one loses the bot.
        if response.refresh_token != current {
            *self.refresh_token.lock().await = response.refresh_token.clone();
            self.store
                .store_refresh_token(channel, &response.refresh_token)
                .await;
        }

        let access = response.access_token;
        *self.token.lock().await = Some(Token {
            access: access.clone(),
            expires_at: Instant::now()
                .checked_add(Duration::from_secs(response.expires_in))
                .unwrap_or_else(|| Instant::now() + REFRESH_MARGIN),
        });

        debug!(
            "Kick: refreshed the token for {} ({}s)",
            channel, response.expires_in
        );

        Ok(access)
    }

    pub async fn send(&self, channel: &str, message: &str) -> Result<(), error::Error> {
        let content = truncate(message);

        let token = self.access_token(channel).await?;
        let status = self.post(&token, &content, channel).await?;

        if status != reqwest::StatusCode::UNAUTHORIZED {
            return Ok(());
        }

        // Fresh-looking but rejected: revoked, or the scope was dropped.
        warn!(
            "Kick: token rejected for {}, renewing and retrying",
            channel
        );
        let token = self.renew(channel).await?;
        if self.post(&token, &content, channel).await? == reqwest::StatusCode::UNAUTHORIZED {
            return Err(error::Error::KickApi(format!(
                "the token for {channel} is still rejected after renewing it, \
                 the authorization was probably revoked"
            )));
        }

        Ok(())
    }

    async fn post(
        &self,
        access_token: &str,
        content: &str,
        channel: &str,
    ) -> Result<reqwest::StatusCode, error::Error> {
        let (kind, broadcaster_user_id) = match self.send_as {
            SendAs::Bot => ("bot", None),
            SendAs::User => ("user", *self.broadcaster_user_id.lock().await),
        };

        if self.send_as == SendAs::User && broadcaster_user_id.is_none() {
            return Err(error::Error::KickApi(format!(
                "sending as a user needs broadcasterUserId in the config of {channel}"
            )));
        }

        let request = ChatMessageRequest {
            content,
            kind,
            broadcaster_user_id,
        };

        let response = self
            .api
            .client
            .post(CHAT_URL)
            .bearer_auth(access_token)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() || status == reqwest::StatusCode::UNAUTHORIZED {
            return Ok(status);
        }

        let body = response.text().await.unwrap_or_default();

        // 500 in bot mode almost always means the OAuth app has no bot account.
        if status == reqwest::StatusCode::INTERNAL_SERVER_ERROR && self.send_as == SendAs::Bot {
            error!(
                "Kick refused to send as a bot in {}. The Kick account that owns \
                 the OAuth app needs a bot account (Settings -> Developer -> your \
                 app); without it this endpoint answers 500. Alternatively set \
                 sendAs: \"user\". Body: {}",
                channel, body
            );
            return Ok(status);
        }

        Err(error::Error::KickApi(format!(
            "sending to {channel} failed with {status}: {body}"
        )))
    }
}

/// Counts characters, not bytes: Kick's limit is in characters and slicing a
/// Rust string mid-codepoint panics.
fn truncate(message: &str) -> String {
    message.chars().take(MAX_MESSAGE_CHARS).collect()
}

/// Resolves a channel's ids from its slug. The chatroom id is not in the public
/// API, so this uses an undocumented endpoint and a failure is never fatal.
pub async fn lookup_channel(client: &reqwest::Client, slug: &str) -> Option<(usize, usize, u64)> {
    #[derive(Deserialize)]
    struct Chatroom {
        id: usize,
    }

    #[derive(Deserialize)]
    struct ChannelResponse {
        id: usize,
        user_id: u64,
        chatroom: Chatroom,
    }

    let request = client
        .get(format!("https://kick.com/api/v2/channels/{slug}"))
        .send();

    let response = match timeout(REQUEST_TIMEOUT, request).await {
        Ok(Ok(response)) if response.status().is_success() => response,
        _ => return None,
    };

    let channel = response.json::<ChannelResponse>().await.ok()?;

    Some((channel.id, channel.chatroom.id, channel.user_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopStore;

    #[async_trait]
    impl KickConfigStore for NoopStore {
        async fn store_refresh_token(&self, _channel: &str, _refresh_token: &str) {}
        async fn store_ids(&self, _channel: &str, _ids: ResolvedIds) {}
    }

    fn api() -> Arc<KickApi> {
        Arc::new(KickApi::new(Credentials {
            client_id: "id".into(),
            client_secret: "secret".into(),
        }))
    }

    #[test]
    fn short_messages_are_untouched() {
        assert_eq!(truncate("!bitrate 5000"), "!bitrate 5000");
    }

    #[test]
    fn long_messages_are_cut_to_the_limit() {
        let truncated = truncate(&"a".repeat(MAX_MESSAGE_CHARS + 50));
        assert_eq!(truncated.chars().count(), MAX_MESSAGE_CHARS);
    }

    #[test]
    fn multi_byte_characters_are_counted_as_one() {
        // Would panic if the message were sliced by bytes.
        let message = "🎥".repeat(MAX_MESSAGE_CHARS + 10);
        let truncated = truncate(&message);

        assert_eq!(truncated.chars().count(), MAX_MESSAGE_CHARS);
        assert!(truncated.len() > MAX_MESSAGE_CHARS);
    }

    #[test]
    fn without_a_refresh_token_there_is_no_sender() {
        let config = config::KickConfig {
            channel_id: Some(1),
            chatroom_id: Some(2),
            ..Default::default()
        };

        assert!(KickSender::new(api(), &config, Arc::new(NoopStore)).is_none());
    }

    #[test]
    fn bot_is_the_default_mode() {
        let config = config::KickConfig {
            refresh_token: Some("token".into()),
            ..Default::default()
        };

        let sender = KickSender::new(api(), &config, Arc::new(NoopStore)).unwrap();
        assert_eq!(sender.send_as, SendAs::Bot);
    }

    #[test]
    fn send_as_user_is_honoured() {
        let config = config::KickConfig {
            refresh_token: Some("token".into()),
            send_as: Some(config::KickSendAs::User),
            ..Default::default()
        };

        let sender = KickSender::new(api(), &config, Arc::new(NoopStore)).unwrap();
        assert_eq!(sender.send_as, SendAs::User);
    }
}
