use crate::{chat::chat_handler, AutomaticSwitchMessage, Noalbs};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    sync::{broadcast, Mutex},
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
    db: Arc<Mutex<HashMap<String, Noalbs>>>,
}

impl Twitch {
    // Please login can't send message as anononemymysouss
    pub fn run(
        config: ClientConfig<StaticLoginCredentials>,
        mut switcher_messages: broadcast::Receiver<AutomaticSwitchMessage>,
        db: Arc<Mutex<HashMap<String, Noalbs>>>,
    ) -> Self {
        let (mut incoming_messages, client) =
            TwitchIRCClient::<TCPTransport, StaticLoginCredentials>::new(config);

        let chat_handler = Arc::new(chat_handler::ChatHandler { db: db.clone() });
        let chat_client = client.clone();
        let reader_handle = tokio::spawn(async move {
            while let Some(message) = incoming_messages.recv().await {
                // println!("Received message: {:?}", message);
                if let ServerMessage::Privmsg(msg) = message {
                    Self::handle_message(&chat_client, msg, &chat_handler).await;
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

                if let Some(user) = &db2.lock().await.get(&sm.channel) {
                    message += &chat_handler::ChatHandler::bitrate(user)
                        .await
                        .to_lowercase();
                }

                let _ = client2.say(sm.channel, message).await;
            }
        });

        Self {
            client,
            reader_handle,
            db,
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
        client: &TwitchIRCClient<TCPTransport, StaticLoginCredentials>,
        message: PrivmsgMessage,
        chat_handler: &chat_handler::ChatHandler,
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
        };

        if let Some(reply) = chat_handler.handle_command(chm).await {
            let _ = client.say(message.channel_login, reply).await;
        }
    }
}
