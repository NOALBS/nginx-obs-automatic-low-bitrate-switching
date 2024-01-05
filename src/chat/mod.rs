use core::fmt;
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
    ServerInfo,
    Otrigger,
    Ortrigger,
    Public,
    Rec,
    Refresh,
    Rtrigger,
    Source,
    Sourceinfo,
    Start,
    Stop,
    Collection,
    Switch,
    Trigger,
    Version,
    LiveScene,
    StartingScene,
    EndingScene,
    PrivacyScene,
    Unknown(String),

    // Internal only
    StopOnRaid(RaidedInfo),
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
            "serverinfo" => Command::ServerInfo,
            "otrigger" => Command::Otrigger,
            "ortrigger" => Command::Ortrigger,
            "public" => Command::Public,
            "record" => Command::Rec,
            "refresh" => Command::Refresh,
            "rtrigger" => Command::Rtrigger,
            "sourceinfo" => Command::Sourceinfo,
            "start" => Command::Start,
            "stop" => Command::Stop,
            "collection" => Command::Collection,
            "switch" => Command::Switch,
            "trigger" => Command::Trigger,
            "source" => Command::Source,

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

#[derive(Debug)]
pub struct CommandPermissions {
    permission: Option<Permission>,
    user_permissions: Option<Vec<String>>,
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
    ES,
    FR,
    IT,
    NB,
    NL,
    PL,
    PTBR,
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
            ChatLanguage::ES => write!(f, "es"),
            ChatLanguage::FR => write!(f, "fr"),
            ChatLanguage::IT => write!(f, "it"),
            ChatLanguage::NB => write!(f, "nb"),
            ChatLanguage::NL => write!(f, "nl"),
            ChatLanguage::PL => write!(f, "pl"),
            ChatLanguage::PTBR => write!(f, "pt_br"),
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
            "es" => Ok(ChatLanguage::ES),
            "fr" => Ok(ChatLanguage::FR),
            "it" => Ok(ChatLanguage::IT),
            "nb" => Ok(ChatLanguage::NB),
            "nl" => Ok(ChatLanguage::NL),
            "pl" => Ok(ChatLanguage::PL),
            "pt_br" => Ok(ChatLanguage::PTBR),
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
    InternalChatUpdate(InternalChatUpdate),
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
pub struct InternalChatUpdate {
    pub platform: ChatPlatform,
    pub channel: String,
    pub kind: InternalUpdate,
}

#[derive(Debug)]
pub enum InternalUpdate {
    Raided(RaidedInfo),
    OfflineTimeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RaidedInfo {
    pub target: String,
    pub display: String,
}

#[derive(Debug)]
pub struct AutomaticSwitchingScene {
    pub platform: ChatPlatform,
    pub channel: String,
    pub scene: String,
    pub switch_type: switcher::SwitchType,
}

#[derive(Debug)]
enum OptionalScene {
    Privacy,
    Starting,
    Ending,
}

impl fmt::Display for OptionalScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptionalScene::Privacy => write!(f, "privacy"),
            OptionalScene::Starting => write!(f, "starting"),
            OptionalScene::Ending => write!(f, "ending"),
        }
    }
}
