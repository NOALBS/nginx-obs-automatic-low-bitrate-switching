use async_trait::async_trait;

pub mod belabox;
pub mod nginx;
pub mod nimble;
pub mod sls;

#[derive(Debug)]
pub enum SwitchType {
    Normal,
    Low,
    Previous,
    Offline,
}

#[async_trait]
pub trait SwitchLogic {
    /// Which scene to switch to
    async fn switch(&self, triggers: &Triggers) -> SwitchType;
}

/// Chat commands
#[async_trait]
pub trait StreamServersCommands {
    async fn bitrate(&self) -> String;
    async fn source_info(&self) -> String;
}

pub trait BSL: SwitchLogic + StreamServersCommands + Send + Sync {}

#[derive(Debug)]
pub struct Triggers {
    /// Trigger to switch to the low scene
    pub low: Option<u32>,

    /// Trigger to switch to the low scene when RTT is high
    pub rtt: Option<u32>,

    /// Trigger to switch to the offline scene
    pub offline: Option<u32>,
}
