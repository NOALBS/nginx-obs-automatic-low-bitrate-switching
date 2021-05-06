use crate::{chat::chat_handler, db::Platform, AutomaticSwitchMessage, Noalbs};
use log::{debug, error, info};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{broadcast, Mutex, RwLock},
    task,
};
use twitch_irc::{
    login::StaticLoginCredentials,
    message::{PrivmsgMessage, ServerMessage},
    ClientConfig, TCPTransport, TwitchIRCClient,
};

pub struct Twitch {
    client: TwitchIRCClient<TCPTransport, StaticLoginCredentials>,
    pub reader_handle: task::JoinHandle<()>,

    // Do i need this?
    _db: Arc<RwLock<HashMap<i64, Noalbs>>>,
}

impl Twitch {
    pub fn run(
        config: ClientConfig<StaticLoginCredentials>,
        mut switcher_messages: broadcast::Receiver<AutomaticSwitchMessage>,
        db: Arc<RwLock<HashMap<i64, Noalbs>>>,
        chat_handler: Arc<chat_handler::ChatHandler>,
    ) -> Self {
        let (mut incoming_messages, client) =
            TwitchIRCClient::<TCPTransport, StaticLoginCredentials>::new(config);

        let chat_client = client.clone();
        let db3 = db.clone();
        let reader_handle = tokio::spawn(async move {
            while let Some(message) = incoming_messages.recv().await {
                // println!("Received message: {:?}", message);
                let cc = chat_client.clone();
                let ch = chat_handler.clone();
                let db = db3.clone();

                match message {
                    ServerMessage::Notice(msg) => {
                        if msg.message_text == "Login authentication failed" {
                            error!("Twitch authentication failed");
                            panic!("Twitch authentication failed");
                        }

                        if msg.message_id == Some("host_on".to_string()) {
                            tokio::spawn(async move {
                                let dbr = db.read().await;
                                let chan = &msg.channel_login.unwrap();
                                let user = dbr
                                    .get(
                                        &ch.username_to_db_user_number(&Platform::Twitch, &chan)
                                            .await,
                                    )
                                    .unwrap();

                                if user.chat_state.lock().await.enable_auto_stop_stream {
                                    log::debug!("Channel started hosting, stopping the stream");
                                    let res = chat_handler::ChatHandler::stop(user).await;
                                    let _ = cc.say(chan.to_string(), res).await;
                                }
                            });
                        }
                    }
                    ServerMessage::Privmsg(msg) => {
                        tokio::spawn(async move {
                            Self::handle_message(cc, msg, ch).await;
                        });
                    }
                    _ => {}
                }
            }
        });

        // Listen for switcher messages to send
        // we should get the state or something here
        // and then construct the message here
        // also need to know the language
        let client2 = client.clone();
        let db2 = db.clone();
        tokio::spawn(async move {
            loop {
                let sm = switcher_messages.recv().await.unwrap();
                log::debug!("Sending automatic switch message to twitch");

                let mut message = format!("Scene switched to \"{}\", ", sm.scene);

                let dbr = &db2.read().await;
                if let Some(user) = &dbr.get(&sm.channel) {
                    message += &chat_handler::ChatHandler::bitrate(user)
                        .await
                        .to_lowercase();

                    if let Some(user) = user
                        .connections
                        .iter()
                        .find(|u| u.platform == Platform::Twitch)
                    {
                        let _ = client2.say(user.channel.to_owned(), message).await;
                    }
                }
            }
        });

        Self {
            client,
            reader_handle,
            _db: db,
        }
    }

    pub fn join<C: Into<String>>(&self, channel: C) {
        self.client.join(channel.into());
    }

    // TODO
    pub fn send_message(&self, message: &str) {
        println!("sending message: {}", message);
    }

    pub async fn handle_message(
        client: TwitchIRCClient<TCPTransport, StaticLoginCredentials>,
        message: PrivmsgMessage,
        chat_handler: Arc<chat_handler::ChatHandler>,
    ) {
        //println!("Received message: {:#?}", message);
        let is_owner = message.badges.contains(&twitch_irc::message::Badge {
            name: "broadcaster".to_string(),
            version: "1".to_string(),
        });

        let is_mod = message.badges.contains(&twitch_irc::message::Badge {
            name: "moderator".to_string(),
            version: "1".to_string(),
        });

        let chm = chat_handler::ChatHandlerMessage {
            message: message.message_text.to_string(),
            channel: message.channel_login.to_string(),
            user: message.sender.login.to_string(),
            is_owner,
            is_mod,
            platform: Platform::Twitch,
        };

        if let Some(reply) = chat_handler.handle_command(chm).await {
            let _ = client.privmsg(message.channel_login, reply).await;
        }
    }
}
