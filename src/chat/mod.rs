use std::fmt::Display;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{error, switcher};

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
    LiveScene,
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

            "live" => Command::LiveScene,
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChatLanguage {
    DE,
    DK,
    EN,
    PL,
    RU,
    SV,
    TR,
    ZHTW,
}

impl Display for ChatLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatLanguage::DE => write!(f, "de"),
            ChatLanguage::DK => write!(f, "dk"),
            ChatLanguage::EN => write!(f, "en"),
            ChatLanguage::PL => write!(f, "pl"),
            ChatLanguage::RU => write!(f, "ru"),
            ChatLanguage::SV => write!(f, "sv"),
            ChatLanguage::TR => write!(f, "tr"),
            ChatLanguage::ZHTW => write!(f, "zh_tw"),
        }
    }
}

impl std::str::FromStr for ChatLanguage {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let language = s.to_lowercase();

        match language.as_ref() {
            "de" => Ok(ChatLanguage::DE),
            "dk" => Ok(ChatLanguage::DK),
            "en" => Ok(ChatLanguage::EN),
            "pl" => Ok(ChatLanguage::PL),
            "ru" => Ok(ChatLanguage::RU),
            "sv" => Ok(ChatLanguage::SV),
            "tr" => Ok(ChatLanguage::TR),
            "zh_tw" => Ok(ChatLanguage::ZHTW),
            _ => Err(error::Error::LangNotSupported),
        }
    }
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
