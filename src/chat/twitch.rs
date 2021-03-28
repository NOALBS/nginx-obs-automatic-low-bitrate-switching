use tokio::{sync::broadcast, task};
use twitch_irc::{
    login::StaticLoginCredentials,
    message::{PrivmsgMessage, ServerMessage},
    ClientConfig, TCPTransport, TwitchIRCClient,
};

use crate::BroadcastMessage;

pub struct Twitch {
    client: TwitchIRCClient<TCPTransport, StaticLoginCredentials>,
    pub reader_handle: task::JoinHandle<()>,
}

impl Twitch {
    // Please login can't send message as anononemymysouss
    pub fn run(
        config: ClientConfig<StaticLoginCredentials>,
        mut switcher_messages: broadcast::Receiver<BroadcastMessage>,
    ) -> Self {
        let (mut incoming_messages, client) =
            TwitchIRCClient::<TCPTransport, StaticLoginCredentials>::new(config);

        // first thing you should do: start consuming incoming messages,
        // otherwise they will back up.
        let reader_handle = tokio::spawn(async move {
            while let Some(message) = incoming_messages.recv().await {
                // println!("Received message: {:?}", message);
                if let ServerMessage::Privmsg(msg) = message {
                    Self::handle_message(msg);
                }
            }
        });

        // Listen for switcher messages to send
        // we should get the state or something here
        // and then construct the message here
        // also need to know the language
        let client2 = client.clone();
        tokio::spawn(async move {
            loop {
                let msg = switcher_messages.recv().await.unwrap();
                let _ = client2.say(msg.channel, msg.message).await;
            }
        });

        //join_handle.await.unwrap();

        Self {
            client,
            reader_handle,
        }
    }

    pub fn join<C: Into<String>>(&self, channel: C) {
        self.client.join(channel.into());
    }

    pub fn handle_message(message: PrivmsgMessage) {
        println!("Received message: {:?}", message);
        //todo!();
    }

    // TODO
    pub fn send_message(&self, message: &str) {
        println!("sending message: {}", message);
    }
}
