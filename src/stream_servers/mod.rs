use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::switcher;

pub mod belabox;
pub mod nginx;
pub mod nimble;
pub mod nms;
pub mod sls;

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait SwitchLogic {
    /// Which scene to switch to
    async fn switch(&self, triggers: &switcher::Triggers) -> switcher::SwitchType;
}

/// Chat commands
#[async_trait]
#[typetag::serde(tag = "type")]
pub trait StreamServersCommands {
    async fn bitrate(&self) -> Bitrate;
    async fn source_info(&self) -> Option<String>;
}

#[typetag::serde(tag = "type")]
pub trait Bsl: SwitchLogic + StreamServersCommands + Send + Sync {}

#[derive(Debug)]
pub struct Bitrate {
    // pub name: &'a str,
    pub message: Option<String>,
    // pub priority: i32,
}

// TODO: This needs a better name
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamServer {
    /// The stream server
    pub stream_server: Box<dyn Bsl>,

    /// A name to differentiate in case of multiple stream servers
    pub name: String,

    /// Priority
    pub priority: Option<i32>,

    /// Override default scenes
    pub override_scenes: Option<switcher::SwitchingScenes>,

    pub depends_on: Option<DependsOn>,

    /// Stream server enabled
    #[serde(default = "default_server_enabled")]
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependsOn {
    pub name: String,
    pub backup_scenes: switcher::SwitchingScenes,
}

fn default_server_enabled() -> bool {
    true
}
