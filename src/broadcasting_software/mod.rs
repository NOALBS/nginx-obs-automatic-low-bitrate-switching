use async_trait::async_trait;

use crate::error::Error;

pub mod obs;
pub mod obs_v5;

#[async_trait]
pub trait BroadcastingSoftwareLogic: Send + Sync {
    async fn switch_scene(&self, scene: &str) -> Result<String, Error>;

    async fn start_streaming(&self) -> Result<(), Error>;

    async fn stop_streaming(&self) -> Result<(), Error>;

    async fn toggle_recording(&self) -> Result<(), Error>;

    async fn is_recording(&self) -> Result<bool, Error>;

    async fn fix(&self) -> Result<(), Error>;

    async fn current_scene(&self) -> Result<String, Error>;

    async fn get_media_source_status(
        &self,
        source_name: &str,
    ) -> Result<(obws::responses::MediaState, i64), Error>;

    async fn create_special_media_source(
        &self,
        source_name: &str,
        scene: &str,
    ) -> Result<String, Error>;

    async fn remove_special_media_source(
        &self,
        source_name: &str,
        scene: &str,
    ) -> Result<(), Error>;
}
