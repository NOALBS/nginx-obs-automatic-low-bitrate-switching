use crate::Noalbs;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ChatHandlerMessage {
    pub message: String,
    pub channel: String,
    pub user: String,
    pub is_owner: bool,
    pub is_mod: bool,
}

pub struct ChatHandler {
    pub db: Arc<RwLock<HashMap<String, Noalbs>>>,
}

impl ChatHandler {
    pub fn new(db: Arc<RwLock<HashMap<String, Noalbs>>>) -> Self {
        Self { db }
    }

    // TODO: Handle permissions per channel and prefix for command
    pub async fn handle_command(&self, msg: ChatHandlerMessage) -> Option<String> {
        dbg!(&msg);

        let mut split_message = msg.message.split_ascii_whitespace();
        let command = split_message.next().unwrap().to_lowercase();

        let dbr = self.db.read().await;
        let user_data = dbr.get(&msg.channel).unwrap();

        Some(match command.as_ref() {
            "!bitrate" => Self::bitrate(&user_data).await,
            "!test" => "it just works".to_string(),
            "!switch" => Self::switch(&user_data, split_message.next()).await,
            "!start" => Self::start(&user_data).await,
            "!stop" => Self::stop(&user_data).await,
            "!noalbs" => Self::noalbs(split_message.next())?,
            "!trigger" => Self::trigger(&user_data, split_message.next()).await,

            "!host" => todo!(),
            "!unhost" => todo!(),
            "!raid" => todo!(),

            "!obsinfo" => todo!(),
            "!refresh" => todo!(),
            "!sourceinfo" => todo!(),
            "!public" => todo!(),
            "!mod" => todo!(),
            "!notify" => todo!(),
            "!autostop" => todo!(),
            "!rec" => todo!(),
            "!fix" => todo!(),
            "!alias" => todo!(),

            _ => return None,
        })
    }

    pub async fn host(data: &Noalbs) -> String {
        todo!();
    }

    pub async fn unhost(data: &Noalbs) -> String {
        todo!();
    }

    pub async fn raid(data: &Noalbs) -> String {
        todo!();
    }

    pub async fn start(data: &Noalbs) -> String {
        match data.broadcasting_software.start_streaming().await {
            Ok(_) => "Successfully started the stream".to_string(),
            Err(error) => {
                format!("Error: {}", error)
            }
        }
    }

    pub async fn stop(data: &Noalbs) -> String {
        match data.broadcasting_software.stop_streaming().await {
            Ok(_) => "Successfully stopped the stream".to_string(),
            Err(error) => {
                format!("Error: {}", error)
            }
        }
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

    // TODO: Make switch smarter
    pub async fn switch(data: &Noalbs, name: Option<&str>) -> String {
        let name = match name {
            Some(name) => name,
            None => return "No scene specified".to_string(),
        };

        match data.broadcasting_software.switch_scene(name).await {
            Ok(_) => {
                format!("Scene successfully switched to \"{}\"", name)
            }
            Err(_) => {
                format!("Can't switch to scene \"{}\"", name)
            }
        }
    }

    // TODO: Safe to file or handle that somewhere else
    pub async fn trigger(data: &Noalbs, value_string: Option<&str>) -> String {
        let value = match value_string {
            Some(name) => name,
            None => {
                let low_trigger = data.switcher_state.lock().await.triggers.low;
                return format!("Current trigger set at {:?} Kbps", low_trigger);
            }
        };

        let value = match value.parse::<u32>() {
            Ok(v) => v,
            Err(_) => return format!("Error editing trigger {} is not a valid value", value),
        };

        let mut state = data.switcher_state.lock().await;
        let real_value = if value == 0 { None } else { Some(value) };
        state.triggers.low = real_value;
        format!("Trigger successfully set to {:?} Kbps", real_value)
    }

    pub fn noalbs(command: Option<&str>) -> Option<String> {
        let command = match command {
            Some(command) => command,
            None => return None,
        };

        match command {
            "version" | "v" => {
                let msg = format!("Running NOALBS v{}", crate::VERSION);
                Some(msg)
            }
            _ => None,
        }
    }
}
