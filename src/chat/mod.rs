use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::switcher;

pub mod chat_handler;
pub mod twitch;

pub use chat_handler::ChatHandler;
pub use twitch::Twitch;

#[async_trait]
pub trait ChatLogic: Send + Sync {
    // TODO: This should return an error
    async fn send_message(&self, channel: String, message: String);
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Command {
    Alias,
    Autostop,
    Bitrate,
    Fix,
    Mod,
    Noalbs,
    Notify,
    Obsinfo,
    Otrigger,
    Public,
    Rec,
    Refresh,
    Rtrigger,
    Sourceinfo,
    Start,
    Stop,
    Switch,
    Trigger,
    Version,
    StartingScene,
    EndingScene,
    PrivacyScene,
    Unknown(String),
}

impl From<&str> for Command {
    fn from(command: &str) -> Self {
        let command = command.to_lowercase();

        match command.as_ref() {
            "alias" => Command::Alias,
            "autostop" => Command::Autostop,
            "bitrate" => Command::Bitrate,
            "fix" => Command::Fix,
            "mod" => Command::Mod,
            "noalbs" => Command::Noalbs,
            "notify" => Command::Notify,
            "obsinfo" => Command::Obsinfo,
            "otrigger" => Command::Otrigger,
            "public" => Command::Public,
            "record" => Command::Rec,
            "refresh" => Command::Refresh,
            "rtrigger" => Command::Rtrigger,
            "sourceinfo" => Command::Sourceinfo,
            "start" => Command::Start,
            "stop" => Command::Stop,
            "switch" => Command::Switch,
            "trigger" => Command::Trigger,

            "noalbsversion" => Command::Version,

            "privacy" => Command::PrivacyScene,
            "starting" => Command::StartingScene,
            "ending" => Command::EndingScene,

            // "host" => Ok(Command::Host),
            // "unhost" => Ok(Command::Unhost),
            // "raid" => Ok(Command::Raid),
            _ => Command::Unknown(command.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Permission {
    Admin,
    Mod,
    Public,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChatPlatform {
    Twitch,
}

#[derive(Debug)]
pub enum HandleMessage {
    ChatMessage(ChatMessage),
    StartedHosting(StartedHosting),
    AutomaticSwitchingScene(AutomaticSwitchingScene),
}

#[derive(Debug)]
pub struct ChatMessage {
    pub platform: ChatPlatform,
    pub permission: Permission,
    pub channel: String,
    pub sender: String,
    pub message: String,
}

#[derive(Debug)]
pub struct StartedHosting {
    pub platform: ChatPlatform,
    pub channel: String,
}

#[derive(Debug)]
pub struct AutomaticSwitchingScene {
    pub platform: ChatPlatform,
    pub channel: String,
    pub scene: String,
    pub switch_type: switcher::SwitchType,
}
