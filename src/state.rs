use std::{collections::HashSet, sync::Arc};

use serde::Serialize;
use tokio::sync::{mpsc, Notify};

use crate::{broadcasting_software::BroadcastingSoftwareLogic, config};

pub struct State {
    pub config: config::Config,
    pub switcher_state: SwitcherState,
    pub broadcasting_software: BroadcastingSoftwareState,
    pub event_senders: Vec<BroadcastClient>,
}

impl State {
    // also should be done once after loading config or adding stream_servers
    pub fn set_all_switchable_scenes(&mut self) {
        let all_scenes = &mut self.switcher_state.switchable_scenes;

        let scenes = &self.config.switcher.switching_scenes;
        all_scenes.insert(scenes.low.to_owned());
        all_scenes.insert(scenes.normal.to_owned());
        all_scenes.insert(scenes.offline.to_owned());

        for servers in &self.config.switcher.stream_servers {
            if let Some(scenes) = &servers.override_scenes {
                all_scenes.insert(scenes.low.to_owned());
                all_scenes.insert(scenes.normal.to_owned());
                all_scenes.insert(scenes.offline.to_owned());
            }

            if let Some(depends_on) = &servers.depends_on {
                let scenes = &depends_on.backup_scenes;
                all_scenes.insert(scenes.low.to_owned());
                all_scenes.insert(scenes.normal.to_owned());
                all_scenes.insert(scenes.offline.to_owned());
            }
        }
    }
}

pub struct SwitcherState {
    pub last_used_server: Option<String>,

    /// All switchable scenes
    pub switchable_scenes: HashSet<String>,

    switcher_enabled_notifier: Arc<Notify>,
}

impl SwitcherState {
    pub fn switcher_enabled_notifier(&self) -> Arc<Notify> {
        self.switcher_enabled_notifier.clone()
    }

    pub async fn wait_till_enabled(&self) {
        self.switcher_enabled_notifier().notified().await;
    }
}

impl Default for SwitcherState {
    fn default() -> Self {
        Self {
            last_used_server: None,
            switcher_enabled_notifier: Arc::new(Notify::new()),
            switchable_scenes: HashSet::new(),
        }
    }
}

pub struct BroadcastingSoftwareState {
    pub prev_scene: String,
    pub current_scene: String,
    pub status: ClientStatus,
    pub is_streaming: bool,
    pub last_stream_started_at: std::time::Instant,
    pub initial_stream_status: Option<StreamStatus>,
    pub stream_status: Option<StreamStatus>,

    // TODO?
    pub connection: Option<Box<dyn BroadcastingSoftwareLogic>>,

    connected_notifier: Arc<Notify>,
    start_streaming_notifier: Arc<Notify>,
    switch_scene_notifier: Arc<Notify>,
}

impl BroadcastingSoftwareState {
    pub fn connected_notifier(&self) -> Arc<Notify> {
        self.connected_notifier.clone()
    }

    pub fn start_streaming_notifier(&self) -> Arc<Notify> {
        self.start_streaming_notifier.clone()
    }

    pub fn switch_scene_notifier(&self) -> Arc<Notify> {
        self.switch_scene_notifier.clone()
    }
}

impl std::fmt::Debug for BroadcastingSoftwareState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BroadcastingSoftwareState")
            .field("prev_scene", &self.prev_scene)
            .field("curent_scene", &self.current_scene)
            .field("status", &self.status)
            .field("is_streaming", &self.is_streaming)
            .field("Does have a software set", &self.connection.is_some())
            .finish()
    }
}

impl Default for BroadcastingSoftwareState {
    fn default() -> Self {
        Self {
            prev_scene: String::new(),
            current_scene: String::new(),
            status: ClientStatus::Disconnected,
            is_streaming: false,
            connection: None,
            connected_notifier: Arc::new(Notify::new()),
            start_streaming_notifier: Arc::new(Notify::new()),
            switch_scene_notifier: Arc::new(Notify::new()),
            last_stream_started_at: std::time::Instant::now(),
            stream_status: None,
            initial_stream_status: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientStatus {
    Connected,
    Disconnected,
}

#[derive(Debug, Default)]
pub struct StreamStatus {
    pub bitrate: u64,
    pub fps: f64,
    pub num_total_frames: u64,
    pub num_dropped_frames: u64,
    pub render_total_frames: u64,
    pub render_missed_frames: u64,
    pub output_total_frames: u64,
    pub output_skipped_frames: u64,
}

impl StreamStatus {
    pub fn calculate_current(&self, old: &Self) -> Self {
        Self {
            bitrate: self.bitrate,
            fps: self.fps,
            num_total_frames: self.num_total_frames - old.num_total_frames,
            num_dropped_frames: self.num_dropped_frames - old.num_dropped_frames,
            render_total_frames: self.render_total_frames - old.render_total_frames,
            render_missed_frames: self.render_missed_frames - old.render_missed_frames,
            output_total_frames: self.output_total_frames - old.output_total_frames,
            output_skipped_frames: self.output_skipped_frames - old.output_skipped_frames,
        }
    }
}

#[derive(Debug)]
pub struct BroadcastClient {
    /// Unique token for the current client
    pub token: String,

    /// Channel used for sending to the websocket
    pub tx_chan: mpsc::UnboundedSender<String>,
}

impl BroadcastClient {
    pub fn send<T>(&self, message: T)
    where
        T: Serialize,
    {
        let json = serde_json::to_string(&message).unwrap();

        if self.tx_chan.send(json).is_err() {
            // Disconnected.. should be handled in reader
        }
    }
}
