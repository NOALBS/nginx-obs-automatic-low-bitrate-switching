use async_trait::async_trait;

use crate::error::Error;

pub mod obs;

#[async_trait]
pub trait BroadcastingSoftwareLogic: Send + Sync {
    async fn switch_scene(&self, scene: &str) -> Result<String, Error>;

    async fn start_streaming(&self) -> Result<(), Error>;

    async fn stop_streaming(&self) -> Result<(), Error>;

    async fn toggle_recording(&self) -> Result<(), Error>;

    async fn is_recording(&self) -> Result<bool, Error>;

    async fn fix(&self) -> Result<(), Error>;
}
