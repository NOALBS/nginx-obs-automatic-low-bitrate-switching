use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{error, info};
use twitch_irc::{
    login::StaticLoginCredentials,
    message,
    transport::tcp::{TCPTransport, TLS},
    ClientConfig, SecureTCPTransport, TwitchIRCClient,
};

use crate::{
    chat::{self, ChatPlatform, HandleMessage},
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

        let event_loop_handle = tokio::spawn(Self::chat_loop(incoming_messages, chat_handler_tx));

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
    ) {
        while let Some(message) = incoming_messages.recv().await {
            // println!("Received message: {:?}", message);

            match message {
                message::ServerMessage::Notice(msg) => {
                    if msg.message_text == "Login authentication failed" {
                        error!("Twitch authentication failed");

                        // TODO: Handle panic
                        // panic!("Twitch authentication failed");
                    }

                    // if msg.message_id == Some("host_on".to_string()) {
                    //     debug!("Channel started hosting, stopping the stream");
                    // }
                }
                message::ServerMessage::HostTarget(host) => {
                    if let message::HostTargetAction::HostModeOn { .. } = host.action {
                        chat_handler_tx
                            .send(HandleMessage::InternalChatUpdate(
                                chat::InternalChatUpdate {
                                    channel: host.channel_login,
                                    platform: ChatPlatform::Twitch,
                                    kind: chat::InternalUpdate::StartedHosting,
                                },
                            ))
                            .await
                            .unwrap();
                    }
                }
                message::ServerMessage::Privmsg(msg) => {
                    let mut permission = chat::Permission::Public;

                    if msg.badges.iter().any(|e| e.name == "moderator") {
                        permission = chat::Permission::Mod;
                    }

                    if msg.badges.iter().any(|e| e.name == "broadcaster") {
                        permission = chat::Permission::Admin;
                    }

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
        self.client.join(channel);
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
