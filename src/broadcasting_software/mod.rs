use async_trait::async_trait;

use crate::error::Error;

pub mod obs;

// #[async_trait]
// #[typetag::serde(tag = "type")]
// pub trait BroadcasintSoftwareConnection: Send + Sync {
//     fn connect() -> dyn BroadcastingSoftwareLogic;
// }

#[async_trait]
pub trait BroadcastingSoftwareLogic: Send + Sync {
    //     fn connected_notifier(&self) -> Arc<Notify>;
    //
    //     async fn wait_to_connect(&self) {
    //         self.connected_notifier().notified().await;
    //     }
    //
    //     fn start_streaming_notifier(&self) -> Arc<Notify>;
    //
    //     /// Waits until OBS starts streaming.
    //     async fn wait_till_streaming(&self) {
    //         self.start_streaming_notifier().notified().await;
    //     }
    //
    //     // fn switching_scenes(&self) -> Arc<Mutex<SwitchingScenes>>;
    //
    //     // async fn can_switch(&self, scene: &str) -> bool {
    //     //     let ss = self.switching_scenes();
    //     //     let switching = ss.lock().await;
    //
    //     //     scene == switching.normal || scene == switching.low || scene == switching.offline
    //     // }
    //
    //     // fn state(&self) -> Arc<Mutex<State>>;
    //
    //     // async fn is_streaming(&self) -> bool {
    //     //     self.state().lock().await.is_streaming
    //     // }
    //
    //     // async fn get_connection_status(&self) -> ClientStatus {
    //     //     self.state().lock().await.status.to_owned()
    //     // }
    //
    //     // async fn is_connected(&self) -> bool {
    //     //     self.get_connection_status().await == ClientStatus::Connected
    //     // }
    //
    //     // async fn get_current_scene(&self) -> String {
    //     //     self.state().lock().await.curent_scene.to_string()
    //     // }
    //
    //     // async fn set_prev_scene(&self, scene: String) {
    //     //     let state = self.state();
    //     //     let mut prev = state.lock().await;
    //     //     prev.prev_scene = scene;
    //     // }
    //
    //     // async fn prev_scene(&self) -> String {
    //     //     self.state().lock().await.prev_scene.to_owned()
    //     // }
    //
    async fn switch_scene(&self, scene: &str) -> Result<String, Error>;
    //
    //     async fn get_scene_list(&self) -> Result<SceneList, Error>;
    //
    async fn start_streaming(&self) -> Result<(), Error>;

    async fn stop_streaming(&self) -> Result<(), Error>;
    //
    //     async fn stream_status(&self) -> Result<StreamingStatus, Error>;
    //
    async fn toggle_recording(&self) -> Result<(), Error>;

    async fn is_recording(&self) -> Result<bool, Error>;

    async fn fix(&self) -> Result<(), Error>;
}

// TODO: What do I actually need
#[derive(Debug)]
pub struct SceneItem {
    pub cy: f64,
    pub cx: f64,
    /// The name of this Scene Item.
    pub name: String,
    /// Scene item ID.
    pub id: i64,
    /// Whether or not this Scene Item is set to "visible".
    pub render: bool,
    /// Whether or not this Scene Item is muted.
    pub muted: bool,
    /// Whether or not this Scene Item is locked and can't be moved around
    pub locked: bool,
}

#[derive(Debug)]
pub struct Scene {
    /// Name of the scene.
    pub name: String,
    /// Ordered list of the scene's source items.
    pub sources: Vec<SceneItem>,
}

#[derive(Debug)]
pub struct SceneList {
    /// Name of the currently active scene.
    pub current_scene: String,
    /// Ordered list of the current profile's scenes.
    pub scenes: Vec<Scene>,
}

#[derive(Debug)]
pub struct StreamingStatus {
    /// Current streaming status.
    pub streaming: bool,
    /// Current recording status.
    pub recording: bool,
    /// If recording is paused.
    pub recording_paused: bool,
}

#[derive(Debug)]
pub struct RecordingStatus {
    /// Current recording status.
    pub is_recording: bool,
    /// Whether the recording is paused or not.
    pub is_recording_paused: bool,
}
