use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{chat, error, stream_servers, switcher};

/// The config of NOALBS
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub user: User,
    pub switcher: Switcher,
    pub software: SoftwareConnection,
    pub chat: Option<Chat>,
    pub optional_scenes: OptionalScenes,
    pub optional_options: OptionalOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Option<i64>,
    pub name: String,
}

/// All the data that can be changed outside of the switcher
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Switcher {
    /// Disable the switcher
    pub bitrate_switcher_enabled: bool,

    /// The interval that the switcher will sleep for before checking the stats again
    pub request_interval: std::time::Duration,

    /// Only enable the switcher when actually streaming from OBS
    pub only_switch_when_streaming: bool,

    /// Enable auto switch chat notification
    pub auto_switch_notification: bool,

    /// Triggers to switch to the low or offline scenes
    pub triggers: switcher::Triggers,

    /// The default switching scenes
    pub switching_scenes: switcher::SwitchingScenes,

    /// Add multiple stream servers to watch before switching to low or offline
    pub stream_servers: Vec<stream_servers::StreamServer>,
}

impl Switcher {
    pub fn add_stream_server(&mut self, stream_server: stream_servers::StreamServer) {
        self.stream_servers.push(stream_server);

        self.sort_stream_servers();
    }

    /// Sort by highest number first
    pub fn sort_stream_servers(&mut self) {
        self.stream_servers
            .sort_by(|a, b| a.priority.cmp(&b.priority));
    }

    pub fn set_bitrate_switcher_enabled(&mut self, enabled: bool) {
        self.bitrate_switcher_enabled = enabled;

        //if enabled {
        //    self.switcher_enabled_notifier.notify_waiters();
        //}
    }
}

impl Default for Switcher {
    fn default() -> Self {
        Self {
            request_interval: std::time::Duration::from_secs(2),
            bitrate_switcher_enabled: true,
            only_switch_when_streaming: true,
            auto_switch_notification: true,
            triggers: switcher::Triggers::default(),
            stream_servers: Vec::new(),
            switching_scenes: switcher::SwitchingScenes {
                normal: "live".to_string(),
                low: "low".to_string(),
                offline: "offline".to_string(),
            },
        }
    }
}

// TODO: Is it possible to do this another way?
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum SoftwareConnection {
    Obs(ObsConfig),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObsConfig {
    pub host: String,
    pub password: Option<String>,
    pub port: u16,
}

pub trait ConfigLogic: Send + Sync {
    fn load(&self) -> Result<Config, error::Error>;
    fn save(&self, config: &Config) -> Result<(), error::Error>;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub platform: chat::ChatPlatform,
    pub username: String,
    pub admins: Vec<String>,

    pub prefix: String,

    pub enable_public_commands: bool,
    pub enable_mod_commands: bool,
    pub enable_auto_stop_stream_on_host_or_raid: bool,
    pub commands: Option<HashMap<chat::Command, CommandInfo>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandInfo {
    pub permission: Option<chat::Permission>,
    pub alias: Option<Vec<String>>,
}

pub struct File {
    pub name: std::path::PathBuf,
}

impl ConfigLogic for File {
    fn load(&self) -> Result<Config, error::Error> {
        let file = std::fs::File::open(&self.name)?;
        let mut config: Config = serde_json::from_reader(file)?;
        config.switcher.sort_stream_servers();

        if let Some(chat) = &mut config.chat {
            chat.username.make_ascii_lowercase();

            for admin in &mut chat.admins {
                admin.make_ascii_lowercase();
            }
        }

        Ok(config)
    }

    // TODO: Handle error
    fn save(&self, config: &Config) -> Result<(), error::Error> {
        let file = std::fs::File::create(&self.name)?;
        serde_json::to_writer_pretty(file, config).unwrap();

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionalScenes {
    pub starting: Option<String>,
    pub ending: Option<String>,
    pub privacy: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionalOptions {
    pub twitch_transcoding_check: bool,
    pub twitch_transcoding_retries: u64,
    pub twitch_transcoding_delay_seconds: u64,
}
