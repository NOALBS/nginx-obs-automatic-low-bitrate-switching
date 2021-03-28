use crate::Noalbs;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ChatHandlerMessage {
    pub message: String,
    pub channel: String,
    pub user: String,
    pub is_owner: bool,
    pub is_mod: bool,
}

pub struct ChatHandler {
    pub db: Arc<Mutex<HashMap<String, Noalbs>>>,
}

impl ChatHandler {
    pub async fn handle_command(&self, msg: ChatHandlerMessage) -> Option<String> {
        dbg!(&msg);

        let mut split_message = msg.message.split_ascii_whitespace();
        let command = split_message.next().unwrap().to_lowercase();

        // Locking the db for every command seems wrong
        let db_lock = self.db.lock().await;
        let user_data = db_lock.get(&msg.channel).unwrap();

        Some(match command.as_ref() {
            "!bitrate" => Self::bitrate(&user_data).await,
            _ => return None,
        })
    }

    pub fn host(&self) -> String {
        todo!();
    }

    pub fn unhost(&self) -> String {
        todo!();
    }

    pub fn raid(&self) -> String {
        todo!();
    }

    pub fn start(&self) -> String {
        todo!();
    }

    pub fn stop(&self) -> String {
        todo!();
    }

    pub async fn bitrate(data: &Noalbs) -> String {
        let mut reply = "Current bitrate".to_string();

        let stats = {
            let mut msg = String::new();

            let servers = &data.switcher_state.lock().await.stream_servers;

            if servers.len() > 1 {
                reply += "s: ";
            } else {
                reply += ": ";
            };

            for (i, s) in servers.iter().enumerate() {
                let t = s.bitrate().await;
                let sep = if i == servers.len() - 1 { "" } else { " & " };
                msg += &format!("{}{}", &t, sep);
            }

            msg
        };

        reply + &stats
    }

    pub fn switch(&self) -> String {
        todo!();
    }
}
