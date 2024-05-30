use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{error, info, trace};
use twitch_irc::{
    login::StaticLoginCredentials,
    message,
    transport::tcp::{TCPTransport, TLS},
    ClientConfig, SecureTCPTransport, TwitchIRCClient,
};

use crate::{
    chat::{self, ChatPlatform, HandleMessage},
    twitch_pubsub::PubsubManager,
    ChatSender,
};

#[derive(Clone)]
pub struct Twitch {
    client: TwitchIRCClient<TCPTransport<TLS>, StaticLoginCredentials>,
    pub event_loop: Arc<tokio::task::JoinHandle<()>>,
}

impl Twitch {
    pub fn new(bot_username: String, mut oauth: String, chat_handler_tx: ChatSender) -> Self {
        if let Some(oauth_without_prefix) = oauth.strip_prefix("oauth:") {
            oauth = oauth_without_prefix.to_string();
        }

        let config = ClientConfig::new_simple(StaticLoginCredentials::new(
            bot_username.to_lowercase(),
            Some(oauth.to_lowercase()),
        ));

        let (incoming_messages, client) =
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

        let pubsub = PubsubManager::new(chat_handler_tx.clone());
        let event_loop_handle =
            tokio::spawn(Self::chat_loop(incoming_messages, chat_handler_tx, pubsub));

        Self {
            client,
            event_loop: Arc::new(event_loop_handle),
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
            // println!("Received message: {:?}", message);

            match message {
                message::ServerMessage::RoomState(state) => {
                    trace!(
                        "user_id: {}, user_name: {}",
                        state.channel_id,
                        state.channel_login
                    );
                    pubsub.add_raid(state.channel_id, state.channel_login).await;
                }
                message::ServerMessage::Notice(msg) => {
                    if msg.message_text == "Login authentication failed" {
                        error!("Twitch authentication failed");

                        // TODO: Handle panic
                        // panic!("Twitch authentication failed");
                    }
                }
                message::ServerMessage::Privmsg(msg) => {
                    let permission =
                        msg.badges
                            .iter()
                            .fold(chat::Permission::Public, |acc, badge| {
                                match badge.name.as_str() {
                                    "vip" => chat::Permission::Vip,
                                    "moderator" => chat::Permission::Mod,
                                    "broadcaster" => chat::Permission::Admin,
                                    _ => acc,
                                }
                            });

                    chat_handler_tx
                        .send(HandleMessage::ChatMessage(chat::ChatMessage {
                            platform: ChatPlatform::Twitch,
                            permission,
                            channel: msg.channel_login,
                            sender: msg.sender.login,
                            message: msg.message_text.to_owned(),
                        }))
                        .await
                        .unwrap();
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
        if let Err(err) = self.client.say(channel, message).await {
            error!("Error sending message to twitch: {}", err);
        }
    }
}

impl Drop for Twitch {
    // Abort the spawned tasks
    fn drop(&mut self) {
        self.event_loop.abort();
    }
}
