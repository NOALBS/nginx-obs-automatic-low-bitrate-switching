use std::{sync::Arc, time::Duration};

use crate::{
    broadcasting_software::BroadcastingSoftwareLogic,
    db, error,
    stream_servers::{Bsl, SwitchType, Triggers},
};
use log::{debug, error, info};
use serde::Serialize;
use tokio::sync::{broadcast, Mutex, Notify, RwLock};

/// All the data that can be changed outside of the switcher
#[derive(Serialize)]
pub struct SwitcherState {
    /// The interval that the switcher will sleep for before checking the stats again
    pub request_interval: std::time::Duration,

    /// Disable the switcher
    pub bitrate_switcher_enabled: bool,

    /// Only enable the switcher when actually streaming from OBS
    pub only_switch_when_streaming: bool,

    /// Enable auto switch chat notfication
    pub auto_switch_notification: bool,

    /// Triggers to switch to the low or offline scenes
    pub triggers: Triggers,

    /// Add multiple stream servers to watch before switching to low or offline
    pub stream_servers: Vec<Box<dyn Bsl>>,

    #[serde(skip_serializing)]
    switcher_enabled_notifier: Arc<Notify>,
}

// TODO: Sort stream_servers with priority
impl SwitcherState {
    pub fn add_stream_server(&mut self, stream_server: Box<dyn Bsl>) {
        self.stream_servers.push(stream_server);
    }

    pub fn set_bitrate_switcher_enabled(&mut self, enabled: bool) {
        self.bitrate_switcher_enabled = enabled;

        if enabled {
            self.switcher_enabled_notifier.notify_waiters();
        }
    }

    fn switcher_enabled_notifier(&self) -> Arc<Notify> {
        self.switcher_enabled_notifier.clone()
    }

    pub async fn wait_till_enabled(&self) {
        self.switcher_enabled_notifier().notified().await;
    }
}

impl Default for SwitcherState {
    fn default() -> Self {
        Self {
            request_interval: Duration::from_secs(2),
            bitrate_switcher_enabled: true,
            only_switch_when_streaming: true,
            auto_switch_notification: true,
            triggers: Triggers::default(),
            stream_servers: Vec::new(),
            switcher_enabled_notifier: Arc::new(Notify::new()),
        }
    }
}

impl From<db::SwitcherState> for SwitcherState {
    fn from(item: db::SwitcherState) -> Self {
        let interval = if item.request_interval < 0 {
            2
        } else {
            item.request_interval as u64
        };

        Self {
            request_interval: Duration::from_secs(interval),
            bitrate_switcher_enabled: item.bitrate_switcher_enabled,
            only_switch_when_streaming: item.only_switch_when_streaming,
            auto_switch_notification: item.auto_switch_notification,
            ..Default::default()
        }
    }
}

pub struct Switcher {
    // Obs etc..
    broadcasting_software: Arc<RwLock<dyn BroadcastingSoftwareLogic>>,

    // TODO: Maybe replace chat with just a Tx so it will send msg's to anyone who's receiving
    // probably also make use of a mpms channel
    //pub chat: Option<Twitch>,
    state: Arc<Mutex<SwitcherState>>,

    notification: broadcast::Sender<AutomaticSwitchMessage>,

    for_channel: db::User,
}

impl Switcher {
    pub fn new(
        for_channel: db::User,
        broadcasting_software: Arc<RwLock<dyn BroadcastingSoftwareLogic>>,
        state: Arc<Mutex<SwitcherState>>,
        notification: broadcast::Sender<AutomaticSwitchMessage>,
    ) -> Self {
        Self {
            broadcasting_software,
            state,
            notification,
            for_channel,
        }
    }

    pub async fn run(self) -> Result<(), error::Error> {
        loop {
            debug!("Running switcher for {}", self.for_channel.username);

            let sleep = { self.state.lock().await.request_interval };
            tokio::time::sleep(sleep).await;

            debug!("Running loop");
            if let Some(notifier) = self.get_sleep_notifier_if_necessary().await {
                notifier.notified().await;
                continue;
            }

            let bs = self.broadcasting_software.read().await;
            let current_scene = bs.get_current_scene().await;
            let can_switch = bs.can_switch(&current_scene).await;
            drop(bs);
            debug!("Can switch: {}", can_switch);
            debug!("Current scene: {}", current_scene);

            if !can_switch {
                continue;
            }

            self.switch().await?;
        }
    }

    async fn get_sleep_notifier_if_necessary(&self) -> Option<Arc<Notify>> {
        let state = self.state.lock().await;

        if !state.bitrate_switcher_enabled {
            info!("Switcher disabled waiting till enabled");
            return Some(state.switcher_enabled_notifier());
        }

        let bs = self.broadcasting_software.read().await;

        if !bs.is_connected().await {
            info!("Waiting for OBS connection");
            return Some(bs.connected_notifier());
        }

        // Yes this will wait even if you change `only_switch_when_streaming`
        if state.only_switch_when_streaming && !bs.is_streaming().await {
            info!("Waiting till OBS starts streaming");
            return Some(bs.start_streaming_notifier());
        }

        None
    }

    /// Returns the type of the first stream server that is not offline
    pub async fn next_switch_type(&self) -> SwitchType {
        let state = &self.state.lock().await;
        let triggers = &state.triggers;

        for s in &state.stream_servers {
            let t = s.switch(&triggers).await;

            if t != SwitchType::Offline {
                return t;
            }
        }

        SwitchType::Offline
    }

    pub async fn switch(&self) -> Result<(), error::Error> {
        let switch_type = self.next_switch_type().await;
        let bs = &self.broadcasting_software.read().await;
        let scene = &bs.type_to_scene(&switch_type).await;

        match switch_type {
            SwitchType::Normal | SwitchType::Low => {
                self.switch_if_necessary(&scene, switch_type).await?;

                let scene = scene.to_owned();
                bs.set_prev_scene(scene).await;
            }
            _ => {
                self.switch_if_necessary(&scene, switch_type).await?;
            }
        };

        Ok(())
    }

    pub async fn switch_if_necessary(
        &self,
        switch_scene: &str,
        switch_type: SwitchType,
    ) -> Result<(), error::Error> {
        let bs = &self.broadcasting_software.read().await;
        let current_scene = bs.get_current_scene().await;

        if current_scene == switch_scene {
            return Ok(());
        }

        // Ignore the error.. it should work at some point
        if let Err(error) = bs.switch_scene(switch_scene).await {
            error!("Switch scene error {:?}", error);
            return Ok(());
        }

        let state = &self.state.lock().await;
        if bs.is_streaming().await && state.auto_switch_notification {
            let _ = self.notification.send(AutomaticSwitchMessage {
                channel: self.for_channel.id,
                scene: switch_scene.to_string(),
                switch_type,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AutomaticSwitchMessage {
    pub channel: i64,
    pub scene: String,
    pub switch_type: SwitchType,
}
