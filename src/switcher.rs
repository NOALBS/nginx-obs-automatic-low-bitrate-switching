use std::{sync::Arc, time::Duration};

use crate::{
    broadcasting_software::obs::Obs,
    chat::twitch::Twitch,
    error,
    stream_servers::{SwitchType, Triggers, BSL},
    AutomaticSwitchMessage,
};
use log::{debug, error, info};
use tokio::sync::{broadcast, mpsc, Mutex, Notify};

/// All the data that can be changed outside of the switcher
pub struct SwitcherState {
    /// The interval that the switcher will sleep for before checking the stats again
    pub request_interval: std::time::Duration,

    /// Disable the switcher
    pub bitrate_switcher_enabled: bool,

    /// Only enable the switcher when actually streaming from OBS
    pub only_switch_when_streaming: bool,

    /// Triggers to switch to the low or offline scenes
    pub triggers: Triggers,

    /// Add multiple stream servers to watch before switching to low or offline
    pub stream_servers: Vec<Box<dyn BSL>>,

    switcher_enabled_notifier: Arc<Notify>,
}

impl SwitcherState {
    pub fn add_stream_server(&mut self, stream_server: Box<dyn BSL>) {
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
            triggers: Triggers::default(),
            stream_servers: Vec::new(),
            switcher_enabled_notifier: Arc::new(Notify::new()),
        }
    }
}

pub struct Switcher {
    // Obs etc..
    broadcasting_software: Arc<Obs>,

    // TODO: Maybe replace chat with just a Tx so it will send msg's to anyone who's receiving
    // probably also make use of a mpms channel
    //pub chat: Option<Twitch>,
    state: Arc<Mutex<SwitcherState>>,

    notification: broadcast::Sender<AutomaticSwitchMessage>,

    for_channel: String,
}

impl Switcher {
    pub fn new<C>(
        for_channel: C,
        broadcasting_software: Arc<Obs>,
        state: Arc<Mutex<SwitcherState>>,
        notification: broadcast::Sender<AutomaticSwitchMessage>,
    ) -> Self
    where
        C: Into<String>,
    {
        let for_channel = for_channel.into();

        Self {
            broadcasting_software,
            state,
            notification,
            for_channel,
        }
    }

    pub async fn run(self) -> Result<(), error::Error> {
        loop {
            let sleep = { self.state.lock().await.request_interval };
            tokio::time::sleep(sleep).await;

            debug!("Running loop");
            if let Some(notifier) = self.get_sleep_notifier_if_necessary().await {
                notifier.notified().await;
                continue;
            }

            let bs = &self.broadcasting_software;
            let current_scene = bs.get_current_scene().await;
            let can_switch = bs.can_switch(&current_scene).await;
            debug!("Can switch: {}", can_switch);
            debug!("Current scene: {}", current_scene);

            if !can_switch {
                continue;
            }

            info!("Running switcher for {}", self.for_channel);
            self.switch().await?;
        }
    }

    async fn get_sleep_notifier_if_necessary(&self) -> Option<Arc<Notify>> {
        let state = self.state.lock().await;

        if !state.bitrate_switcher_enabled {
            info!("Switcher disabled waiting till enabled");
            return Some(state.switcher_enabled_notifier());
        }

        if !self.broadcasting_software.is_connected().await {
            info!("Waiting for OBS connection");
            return Some(self.broadcasting_software.connected_notifier());
        }

        // Yes this will wait even if you change `only_switch_when_streaming`
        if state.only_switch_when_streaming && !self.broadcasting_software.is_streaming().await {
            info!("Waiting till OBS starts streaming");
            return Some(self.broadcasting_software.start_streaming_notifier());
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
        let switch = self.next_switch_type().await;
        let scene = &self.broadcasting_software.type_to_scene(&switch).await;

        match switch {
            SwitchType::Normal | SwitchType::Low => {
                self.switch_if_necessary(&scene).await?;

                let scene = scene.to_owned();
                self.broadcasting_software.set_prev_scene(scene).await;
            }
            _ => {
                self.switch_if_necessary(&scene).await?;
            }
        };

        Ok(())
    }

    pub async fn switch_if_necessary(&self, switch_scene: &str) -> Result<(), error::Error> {
        let bs = &self.broadcasting_software;
        let current_scene = bs.get_current_scene().await;

        if current_scene == switch_scene {
            return Ok(());
        }

        // Ignore the error.. it should work at some point
        if let Err(error) = bs.switch_scene(switch_scene).await {
            error!("Switch scene error {:?}", error);
            return Ok(());
        }

        if bs.is_streaming().await {
            let _ = self.notification.send(AutomaticSwitchMessage {
                channel: self.for_channel.to_string(),
                scene: switch_scene.to_string(),
            });
        }

        Ok(())
    }
}
