use std::sync::Arc;

use crate::{
    error,
    stream_servers::{self, SwitchType},
    Error,
};
use async_trait::async_trait;
use tokio::sync::{Mutex, Notify};

pub mod obs;

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct SwitchingScenes {
    pub normal: String,
    pub low: String,
    pub offline: String,
}

impl SwitchingScenes {
    pub fn new<N, L, O>(normal: N, low: L, offline: O) -> Self
    where
        N: Into<String>,
        L: Into<String>,
        O: Into<String>,
    {
        SwitchingScenes {
            normal: normal.into(),
            low: low.into(),
            offline: offline.into(),
        }
    }

    pub fn type_to_scene(&self, s_type: &SwitchType) -> Result<String, error::Error> {
        let str = match s_type {
            SwitchType::Normal => &self.normal,
            SwitchType::Low => &self.low,
            SwitchType::Offline => &self.offline,
            _ => return Err(error::Error::SwitchTypeNotSupported),
        };

        Ok(str.to_string())
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Config {
    /// The hostname
    pub host: String,
    /// The password
    pub password: String,
    /// Port to connect to
    pub port: u16,
}

#[derive(Debug)]
pub struct State {
    prev_scene: String,
    curent_scene: String,
    status: ClientStatus,
    is_streaming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientStatus {
    Connected,
    Disconnected,
}

#[async_trait]
pub trait BroadcastingSoftwareLogic: Send + Sync {
    fn connected_notifier(&self) -> Arc<Notify>;

    async fn wait_to_connect(&self) {
        self.connected_notifier().notified().await;
    }

    fn start_streaming_notifier(&self) -> Arc<Notify>;

    /// Waits until OBS starts streaming.
    async fn wait_till_streaming(&self) {
        self.start_streaming_notifier().notified().await;
    }

    fn switching_scenes(&self) -> Arc<Mutex<SwitchingScenes>>;

    async fn can_switch(&self, scene: &str) -> bool {
        let ss = self.switching_scenes();
        let switching = ss.lock().await;

        scene == switching.normal || scene == switching.low || scene == switching.offline
    }

    async fn type_to_scene(&self, s_type: &stream_servers::SwitchType) -> String {
        let ss = self.switching_scenes();
        let switching = ss.lock().await;

        match s_type {
            // Safety: Should be safe to unwrap since we are handling the previous.
            SwitchType::Normal | SwitchType::Low | SwitchType::Offline => {
                switching.type_to_scene(s_type).unwrap()
            }
            SwitchType::Previous => self.prev_scene().await,
        }
    }

    fn state(&self) -> Arc<Mutex<State>>;

    async fn is_streaming(&self) -> bool {
        self.state().lock().await.is_streaming
    }

    async fn get_connection_status(&self) -> ClientStatus {
        self.state().lock().await.status.to_owned()
    }

    async fn is_connected(&self) -> bool {
        self.get_connection_status().await == ClientStatus::Connected
    }

    async fn get_current_scene(&self) -> String {
        self.state().lock().await.curent_scene.to_string()
    }

    async fn set_prev_scene(&self, scene: String) {
        let state = self.state();
        let mut prev = state.lock().await;
        prev.prev_scene = scene;
    }

    async fn prev_scene(&self) -> String {
        self.state().lock().await.prev_scene.to_owned()
    }

    async fn switch_scene(&self, scene: &str) -> Result<(), Error>;

    async fn get_scene_list(&self) -> Result<SceneList, Error>;

    async fn start_streaming(&self) -> Result<(), Error>;

    async fn stop_streaming(&self) -> Result<(), Error>;

    async fn stream_status(&self) -> Result<StreamingStatus, Error>;

    async fn toggle_recording(&self) -> Result<(), Error>;

    async fn recording_status(&self) -> Result<RecordingStatus, Error>;
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
