use crate::{db, error::Error, stream_servers, Noalbs};
use db::Platform;
use log::error;
use std::{collections::HashMap, sync::Arc};
use stream_servers::TriggerType;
use tokio::sync::RwLock;

#[derive(Debug)]
pub enum SupportedChat {
    Twitch,
}

#[derive(Debug, PartialEq, Eq, Hash, sqlx::Type, Clone, Copy)]
#[sqlx(rename_all = "lowercase")]
pub enum Command {
    Test,
    Bitrate,
    Switch,
    Start,
    Stop,
    Noalbs,
    Trigger,
    Otrigger,
    Rtrigger,
    Obsinfo,
    Refresh,
    Sourceinfo,
    Public,
    Mod,
    Notify,
    Autostop,
    Rec,
    Fix,
    Alias,

    // Where should i put platform specific
    Host,
    Unhost,
    Raid,
}

impl std::str::FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "test" => Ok(Command::Test),
            "bitrate" => Ok(Command::Bitrate),
            "switch" => Ok(Command::Switch),
            "start" => Ok(Command::Start),
            "stop" => Ok(Command::Stop),
            "noalbs" => Ok(Command::Noalbs),
            "trigger" => Ok(Command::Trigger),
            "otrigger" => Ok(Command::Otrigger),
            "rtrigger" => Ok(Command::Obsinfo),
            "obsinfo" => Ok(Command::Obsinfo),
            "refresh" => Ok(Command::Refresh),
            "sourceinfo" => Ok(Command::Sourceinfo),
            "public" => Ok(Command::Public),
            "mod" => Ok(Command::Mod),
            "notify" => Ok(Command::Notify),
            "autostop" => Ok(Command::Autostop),
            "rec" => Ok(Command::Rec),
            "fix" => Ok(Command::Fix),
            "alias" => Ok(Command::Alias),
            _ => Err(format!("'{}' is not a valid command", s)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum Permission {
    Moderator,
    Public,
}

impl Permission {
    /// All the public/moderator commands
    ///
    /// Note: Does not include admin commands since they can already use all
    /// the commands
    pub fn default_permissions() -> HashMap<Command, Permission> {
        let mut permissions = HashMap::new();

        permissions.insert(Command::Refresh, Permission::Moderator);
        permissions.insert(Command::Fix, Permission::Moderator);
        permissions.insert(Command::Trigger, Permission::Moderator);
        permissions.insert(Command::Rtrigger, Permission::Moderator);
        permissions.insert(Command::Otrigger, Permission::Moderator);
        permissions.insert(Command::Sourceinfo, Permission::Moderator);
        permissions.insert(Command::Obsinfo, Permission::Moderator);
        permissions.insert(Command::Bitrate, Permission::Public);

        permissions
    }
}

#[derive(Debug)]
pub struct ChatHandlerMessage {
    pub message: String,
    pub channel: String,
    pub user: String,
    pub is_owner: bool,
    pub is_mod: bool,
    pub platform: Platform,
}

pub struct ChatHandler {
    pub db: Arc<RwLock<HashMap<i64, Noalbs>>>,
    default_permissions: HashMap<Command, Permission>,
}

impl ChatHandler {
    pub fn new(db: Arc<RwLock<HashMap<i64, Noalbs>>>) -> Self {
        Self {
            db,
            default_permissions: Permission::default_permissions(),
        }
    }

    // TODO: Handle permissions per channel and prefix for command
    pub async fn handle_command(&self, msg: ChatHandlerMessage) -> Option<String> {
        dbg!(&msg);

        // Get the current channel settings from the database
        let dbr = self.db.read().await;
        let user_data = dbr
            .iter()
            .find_map(|(_, b)| {
                if b.connections
                    .iter()
                    .any(|u| u.platform == msg.platform && u.channel == msg.channel)
                {
                    Some(b)
                } else {
                    None
                }
            })
            .unwrap();

        let udcs_lock = user_data.chat_state.lock().await;
        let custom_permissions = &udcs_lock.commands_permissions;
        let prefix = &udcs_lock.prefix;

        if msg.message.is_empty() || !msg.message.starts_with(prefix) {
            return None;
        }

        let mut split_message = msg
            .message
            .strip_prefix(prefix)
            .unwrap()
            .split_ascii_whitespace();

        // unwrap should be safe since there are no empty messages and already
        // checked if message starts with prefix
        let command = split_message.next().unwrap().to_lowercase();

        // Check aliases
        let command = if let Some(command) = udcs_lock.commands_aliases.get(&command) {
            command.to_owned()
        } else {
            match command.parse() {
                Ok(command) => command,
                Err(_) => return None,
            }
        };

        if !msg.is_owner {
            if custom_permissions.contains_key(&command) {
                if !Self::is_allowed_to_run_command(&custom_permissions, &msg, &command) {
                    return None;
                }
            } else if !Self::is_allowed_to_run_command(&self.default_permissions, &msg, &command) {
                return None;
            }
        }

        // Drop the lock since it's no longer needed
        drop(udcs_lock);

        // First check if command is platform specific
        match msg.platform {
            Platform::Twitch => {
                if let Some(msg) =
                    TwitchChatHandler::handle_command(&command, split_message.by_ref()).await
                {
                    return Some(msg);
                }
            }
            Platform::Youtube => {}
        }

        Some(match command {
            Command::Test => "hello test".to_string(),
            Command::Bitrate => Self::bitrate(&user_data).await,
            Command::Switch => Self::switch(&user_data, split_message.next()).await,
            Command::Start => Self::start(&user_data).await,
            Command::Stop => Self::stop(&user_data).await,
            Command::Noalbs => {
                Self::noalbs(&user_data, split_message.next(), split_message).await?
            }
            Command::Trigger => {
                Self::trigger(&user_data, TriggerType::Low, split_message.next()).await
            }
            Command::Otrigger => {
                Self::trigger(&user_data, TriggerType::Offline, split_message.next()).await
            }
            Command::Rtrigger => {
                Self::trigger(&user_data, TriggerType::Rtt, split_message.next()).await
            }
            Command::Obsinfo => todo!(),
            Command::Refresh => todo!(),
            Command::Sourceinfo => todo!(),
            Command::Public => todo!(),
            Command::Mod => todo!(),
            Command::Notify => Self::notify(&user_data, split_message.next()).await,
            Command::Autostop => todo!(),
            Command::Rec => todo!(),
            Command::Fix => todo!(),
            Command::Alias => todo!(),
            _ => return None,
        })
    }

    fn is_allowed_to_run_command(
        permissions: &HashMap<Command, Permission>,
        msg: &ChatHandlerMessage,
        command: &Command,
    ) -> bool {
        if let Some(permission) = permissions.get(&command) {
            if permission == &Permission::Public
                || (permission == &Permission::Moderator && msg.is_mod)
            {
                return true;
            }
        }

        false
    }

    pub async fn start(data: &Noalbs) -> String {
        match data.broadcasting_software.start_streaming().await {
            Ok(_) => "Successfully started the stream".to_string(),
            Err(error) => {
                error!("Error: {}", error);
                "Stream already started or no connection to OBS".to_string()
            }
        }
    }

    pub async fn stop(data: &Noalbs) -> String {
        match data.broadcasting_software.stop_streaming().await {
            Ok(_) => "Successfully stopped the stream".to_string(),
            Err(error) => {
                error!("Error: {}", error);
                "Stream already stopped or no connection to OBS".to_string()
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

    async fn get_trigger(data: &Noalbs, kind: stream_servers::TriggerType) -> Option<u32> {
        let triggers = &data.switcher_state.lock().await.triggers;
        dbg!(&triggers);

        match kind {
            TriggerType::Low => triggers.low,
            TriggerType::Rtt => triggers.rtt,
            TriggerType::Offline => triggers.offline,
        }
    }

    async fn update_trigger(
        data: &Noalbs,
        kind: stream_servers::TriggerType,
        value: u32,
    ) -> String {
        let mut state = data.switcher_state.lock().await;
        let real_value = if value == 0 { None } else { Some(value) };

        match kind {
            TriggerType::Low => state.triggers.low = real_value,
            TriggerType::Rtt => state.triggers.rtt = real_value,
            TriggerType::Offline => state.triggers.offline = real_value,
        }

        format!("Trigger successfully set to {:?} Kbps", real_value)
    }

    // TODO: Save to file or handle that somewhere else
    pub async fn trigger(
        data: &Noalbs,
        kind: stream_servers::TriggerType,
        value_string: Option<&str>,
    ) -> String {
        let value = match value_string {
            Some(name) => name,
            None => {
                return format!(
                    "Current trigger set at {:?} Kbps",
                    Self::get_trigger(data, kind).await
                );
            }
        };

        let value = match value.parse::<u32>() {
            Ok(v) => v,
            Err(_) => return format!("Error editing trigger {} is not a valid value", value),
        };

        Self::update_trigger(data, kind, value).await
    }

    pub async fn noalbs<'a, I>(data: &Noalbs, command: Option<&str>, args: I) -> Option<String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let command = match command {
            Some(command) => command,
            None => return None,
        };

        let mut args = args.into_iter();

        match command {
            "version" | "v" => Some(format!("Running NOALBS v{}", crate::VERSION)),
            "prefix" => {
                if let Some(prefix) = args.next() {
                    Self::set_prefix(data, prefix.to_owned()).await;
                    Some(format!("NOALBS prefix updated to {}", prefix))
                } else {
                    None
                }
            }
            "start" => {
                Self::set_bitrate_switcher_state(data, true).await;
                Some("Successfully enabled the switcher".to_string())
            }
            "stop" => {
                Self::set_bitrate_switcher_state(data, false).await;
                Some("Successfully disabled the switcher".to_string())
            }
            _ => None,
        }
    }

    pub async fn set_bitrate_switcher_state(data: &Noalbs, enabled: bool) {
        let mut lock = data.switcher_state.lock().await;
        lock.set_bitrate_switcher_enabled(enabled);
    }

    pub async fn set_prefix(data: &Noalbs, prefix: String) {
        let mut lock = data.chat_state.lock().await;
        lock.prefix = prefix;
    }

    pub async fn notify(data: &Noalbs, enabled: Option<&str>) -> String {
        let mut lock = data.switcher_state.lock().await;
        let asn = "Auto switch notification";

        if let Some(enabled) = enabled {
            if let Ok(b) = enabled_to_bool(enabled) {
                lock.auto_switch_notification = b;

                return if b {
                    format!("{} enabled", asn)
                } else {
                    format!("{} disabled", asn)
                };
            }
        }

        format!(
            "{} is {}",
            asn,
            if lock.auto_switch_notification {
                "enabled"
            } else {
                "disabled"
            }
        )
    }
}

fn enabled_to_bool(enabled: &str) -> Result<bool, Error> {
    if enabled.to_lowercase() == "on" {
        return Ok(true);
    }

    if enabled.to_lowercase() == "off" {
        return Ok(false);
    }

    Err(Error::EnabledToBoolConversionError)
}

struct TwitchChatHandler {}
impl TwitchChatHandler {
    pub async fn handle_command<'a, I>(command: &Command, args: I) -> Option<String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut args = args.into_iter();

        Some(match command {
            Command::Host => format!("/host {}", args.next()?),
            Command::Unhost => "/unhost".to_string(),
            Command::Raid => format!("/raid {}", args.next()?),
            _ => return None,
        })
    }
}
