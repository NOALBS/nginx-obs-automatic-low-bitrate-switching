use std::sync::Arc;
use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{error, info, trace, warn};
use twitch_irc::{
    ClientConfig, SecureTCPTransport, TwitchIRCClient,
    login::StaticLoginCredentials,
    message,
    transport::tcp::{TCPTransport, TLS},
};

use crate::{
    ChatSender,
    chat::{self, ChatPlatform, HandleMessage},
    twitch_pubsub::PubsubManager,
};

#[derive(Clone)]
pub struct Twitch {
    client: TwitchIRCClient<TCPTransport<TLS>, StaticLoginCredentials>,
    pub event_loop: Arc<tokio::task::JoinHandle<()>>,
    client_id: String,          // Client-Id matching helix_access_token
    helix_access_token: String, // App or User token for Helix
    bot_user_id: String,        // sender_id in the Helix payload
    channel_ids: HashMap<String, String>, // broadcaster_login -> broadcaster_id
    http_client: reqwest::Client,
    helix_ok: bool,
    use_app_token: bool,        // true => attempt badge mode (requires MOD in channel or channel:bot)
}

impl Twitch {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bot_username: String,
        oauth_for_irc: String, // expects token WITHOUT "oauth:" prefix
        chat_handler_tx: ChatSender,
        client_id: String,
        helix_access_token: String,
        bot_user_id: String,
        channel_ids: HashMap<String, String>,
        helix_ok: bool,
        use_app_token: bool,
    ) -> Self {
        let config = ClientConfig::new_simple(StaticLoginCredentials::new(
            bot_username.to_lowercase(),
            Some(oauth_for_irc),
        ));

        let (incoming_messages, client) =
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

        let pubsub = PubsubManager::new(chat_handler_tx.clone());
        let event_loop_handle =
            tokio::spawn(Self::chat_loop(incoming_messages, chat_handler_tx, pubsub));

        Self {
            client,
            event_loop: Arc::new(event_loop_handle),
            client_id,
            helix_access_token,
            bot_user_id,
            channel_ids,
            http_client: reqwest::Client::new(),
            helix_ok,
            use_app_token,
        }
    }

    pub fn get_client(&self) -> TwitchIRCClient<TCPTransport<TLS>, StaticLoginCredentials> {
        self.client.clone()
    }

    async fn chat_loop(
        mut incoming_messages: mpsc::UnboundedReceiver<message::ServerMessage>,
        chat_handler_tx: tokio::sync::mpsc::Sender<super::HandleMessage>,
        pubsub: PubsubManager,
    ) {
        while let Some(message) = incoming_messages.recv().await {
            match message {
                message::ServerMessage::RoomState(state) => {
                    trace!("user_id: {}, user_name: {}", state.channel_id, state.channel_login);
                    pubsub.add_raid(state.channel_id, state.channel_login).await;
                }
                message::ServerMessage::Notice(msg) => {
                    if msg.message_text == "Login authentication failed" {
                        error!("Twitch authentication failed");
                    } else {
                        trace!("NOTICE: {}", msg.message_text);
                    }
                }
                message::ServerMessage::Privmsg(msg) => {
                    let permission = msg.badges.iter().fold(chat::Permission::Public, |acc, badge| {
                        match badge.name.as_str() {
                            "vip" => chat::Permission::Vip,
                            "moderator" => chat::Permission::Mod,
                            "broadcaster" => chat::Permission::Admin,
                            _ => acc,
                        }
                    });

                    let _ = chat_handler_tx.send(HandleMessage::ChatMessage(chat::ChatMessage {
                        platform: ChatPlatform::Twitch,
                        permission,
                        channel: msg.channel_login,
                        sender: msg.sender.login,
                        message: msg.message_text.to_owned(),
                    })).await;
                }
                _ => {}
            }
        }
    }

    pub fn join_channel(&self, channel: String) {
        info!("Joining channel: {}", channel);
        if let Err(e) = self.client.join(channel) {
            error!("Error joining channel: {}", e);
        }
    }
}

#[async_trait]
impl super::ChatLogic for Twitch {
    async fn send_message(&self, channel: String, message: String) {
        let channel_login = channel.to_lowercase();

        // Try Helix first
        if self.helix_ok && !self.client_id.is_empty() && !self.helix_access_token.is_empty() && !self.bot_user_id.is_empty() {
            if let Some(broadcaster_id) = self.channel_ids.get(&channel_login) {
                let url = "https://api.twitch.tv/helix/chat/messages";

                // With APP token we set for_source_only=false to ensure the message goes to shared chat too.
                // NOTE: this parameter is illegal with User token (400), so we include it only in app mode. (See API ref.)
                // https://dev.twitch.tv/docs/api/reference/#send-chat-message
                let payload = if self.use_app_token {
                    serde_json::json!({
                        "broadcaster_id": broadcaster_id,
                        "sender_id": self.bot_user_id,
                        "message": message,
                        "for_source_only": true
                    })
                } else {
                    serde_json::json!({
                        "broadcaster_id": broadcaster_id,
                        "sender_id": self.bot_user_id,
                        "message": message
                    })
                };

                let result = self.http_client
                    .post(url)
                    .header("Client-Id", &self.client_id)
                    .header("Authorization", format!("Bearer {}", self.helix_access_token))
                    .json(&payload)
                    .send()
                    .await;

                match result {
                    Ok(resp) => {
                        if !resp.status().is_success() {
                            let status = resp.status();
                            let err_text = resp.text().await.unwrap_or_default();
                            error!("Helix POST /chat/messages failed ({}): {}", status, err_text);
                            if self.use_app_token {
                                // Common badge-mode causes: not MOD in the channel, or missing channel:bot grant from broadcaster.
                                warn!("Tip: For bot badge the bot must be MOD in the channel OR the app must have 'channel:bot' from the broadcaster. Otherwise Twitch returns 403.");
                            }
                            // Fallback to IRC
                            if let Err(err) = self.client.say(
                                channel_login.clone(),
                                payload["message"].as_str().unwrap_or_default().to_string()
                            ).await {
                                error!("IRC fallback failed: {}", err);
                            }
                        }
                    }
                    Err(err) => {
                        error!("Helix error: {}", err);
                        if self.use_app_token {
                            warn!("(Badge mode) Check that the app token is valid and the bot is MOD or has 'channel:bot'.");
                        }
                        // Fallback to IRC
                        if let Err(err2) = self.client.say(channel_login.clone(), message.clone()).await {
                            error!("IRC fallback failed: {}", err2);
                        }
                    }
                }
                return;
            } else {
                warn!("Missing broadcaster_id for '{}', sending via IRC (no bot badge).", channel_login);
            }
        } else {
            trace!("Helix disabled or missing fields — sending via IRC (no bot badge).");
        }

        // IRC (no badge)
        if let Err(err) = self.client.say(channel_login, message).await {
            error!("IRC send failed: {}", err);
        }
    }
}

impl Drop for Twitch {
    fn drop(&mut self) { self.event_loop.abort(); }
}
