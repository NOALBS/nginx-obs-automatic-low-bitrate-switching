use async_trait::async_trait;
use serde::Serialize;

pub mod belabox;
pub mod nginx;
pub mod nimble;
pub mod sls;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SwitchType {
    Normal,
    Low,
    Previous,
    Offline,
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait SwitchLogic {
    /// Which scene to switch to
    async fn switch(&self, triggers: &Triggers) -> SwitchType;
}

/// Chat commands
#[async_trait]
#[typetag::serde(tag = "type")]
pub trait StreamServersCommands {
    async fn bitrate(&self) -> Bitrate;
    async fn source_info(&self) -> String;
}

#[typetag::serde(tag = "type")]
pub trait Bsl: SwitchLogic + StreamServersCommands + Send + Sync {}

#[derive(Debug)]
pub struct Bitrate<'a> {
    pub name: &'a str,
    pub message: Option<String>,
}

#[derive(Debug)]
pub enum TriggerType {
    Low,
    Rtt,
    Offline,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Triggers {
    /// Trigger to switch to the low scene
    pub low: Option<u32>,

    /// Trigger to switch to the low scene when RTT is high
    pub rtt: Option<u32>,

    /// Trigger to switch to the offline scene
    pub offline: Option<u32>,
}

impl Triggers {
    pub fn set_low(&mut self, value: Option<u32>) {
        self.low = value;
    }
}

impl Default for Triggers {
    fn default() -> Self {
        Self {
            low: Some(800),
            rtt: Some(2500),
            offline: None,
        }
    }
}
