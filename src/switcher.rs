use std::sync::Arc;

use crate::{
    broadcasting_software::obs::Obs,
    chat::twitch::Twitch,
    error,
    stream_servers::{SwitchType, Triggers, BSL},
};
use log::{debug, error, info};
use tokio::sync::Mutex;

pub struct Switcher {
    // Nginx, Srt-live... etc
    pub stream_server: Box<dyn BSL>,

    // Obs etc..
    pub broadcasting_software: Arc<Obs>,

    // TODO: Maybe replace chat with just a Tx so it will send msg's to anyone who's receiving
    // probably also make use of a mpms channel
    pub chat: Option<Twitch>,

    pub state: Arc<Mutex<SwitcherState>>,
}

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
}

impl Switcher {
    pub async fn run(self) -> Result<(), error::Error> {
        loop {
            let mut state = self.state.lock().await;

            tokio::time::sleep(state.request_interval).await;
            debug!("Running loop");

            if !self.broadcasting_software.is_connected().await {
                // Drop the mutex since this could take a long time and
                // it should still be possible to change the state.
                drop(state);

                debug!("Loop waiting for OBS connection before continuing");
                self.broadcasting_software.wait_to_connect().await;

                state = self.state.lock().await;
            }

            if !state.bitrate_switcher_enabled {
                continue;
            }

            if state.only_switch_when_streaming {
                if !self.broadcasting_software.is_streaming().await {
                    debug!("Not streaming from OBS");
                    continue;
                }
            }

            drop(state);

            let bs = &self.broadcasting_software;
            let current_scene = bs.get_current_scene().await;
            let can_switch = bs.can_switch(&current_scene);
            debug!("Can switch: {}", can_switch);
            debug!("Current scene: {}", current_scene);

            if !can_switch {
                continue;
            }

            self.switch().await?;
        }
    }

    pub async fn next_switch_type(&self) -> SwitchType {
        let triggers = &self.state.lock().await.triggers;
        self.stream_server.switch(&triggers).await
    }

    pub async fn switch(&self) -> Result<(), error::Error> {
        let switch = self.next_switch_type().await;

        match switch {
            SwitchType::Normal => {
                let scene = &self.broadcasting_software.switching.normal;
                self.switch_if_necessary(&scene).await?;

                let scene = scene.to_owned();
                self.broadcasting_software.set_prev_scene(scene).await;
            }
            SwitchType::Low => {
                let scene = &self.broadcasting_software.switching.low;
                self.switch_if_necessary(&scene).await?;

                let scene = scene.to_owned();
                self.broadcasting_software.set_prev_scene(scene).await;
            }
            SwitchType::Previous => {
                let scene = &self.broadcasting_software.prev_scene().await;
                self.switch_if_necessary(&scene).await?;
            }
            SwitchType::Offline => {
                let scene = &self.broadcasting_software.switching.offline;
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

        let msg = format!(
            "Switch to: {:?}, current stats: {}",
            switch_scene,
            self.stream_server.bitrate().await
        );
        info!("{}", msg);

        if let Some(chat) = &self.chat {
            chat.send_message(&msg);
        }

        Ok(())
    }
}
