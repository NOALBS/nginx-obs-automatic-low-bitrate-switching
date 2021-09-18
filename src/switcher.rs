use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tracing::{debug, error, info, Instrument};

use crate::{
    chat, error,
    noalbs::{self, ChatSender},
    state::ClientStatus,
    stream_servers,
};

pub struct Switcher {
    pub state: noalbs::UserState,
    pub chat_sender: ChatSender,
}

impl Switcher {
    pub fn run(switcher: Self) -> tokio::task::JoinHandle<()> {
        tracing::info!("Running switcher");

        let f = async move {
            loop {
                tokio::time::sleep({
                    let state = switcher.state.read().await;
                    state.config.switcher.request_interval
                })
                .await;

                tracing::debug!("Switcher loop");

                if let Some(notifier) = switcher.get_sleep_notifier_if_necessary().await {
                    notifier.notified().await;
                    continue;
                }

                if let Err(e) = switcher.switch().await {
                    error!("Error when trying to switch: {}", e);
                }
            }
        }
        .instrument(tracing::info_span!("Switcher"));

        tokio::spawn(f)
    }

    pub async fn get_sleep_notifier_if_necessary(&self) -> Option<Arc<Notify>> {
        let state = self.state.read().await;

        if !state.config.switcher.bitrate_switcher_enabled {
            info!("Switcher disabled waiting till enabled");
            return Some(state.switcher_state.switcher_enabled_notifier());
        }

        if state.broadcasting_software.status == ClientStatus::Disconnected {
            info!("Waiting for OBS connection");
            return Some(state.broadcasting_software.connected_notifier());
        }

        // TODO: When changing only_switch_when_streaming also do a
        // notify so that it won't wait anymore
        if state.config.switcher.only_switch_when_streaming
            && !state.broadcasting_software.is_streaming
        {
            info!("Waiting till OBS starts streaming");
            return Some(state.broadcasting_software.start_streaming_notifier());
        }

        if !state
            .switcher_state
            .switchable_scenes
            .contains(&state.broadcasting_software.current_scene)
        {
            info!("Not able to switch, waiting for scene switch to a switchable scene");
            return Some(state.broadcasting_software.switch_scene_notifier());
        }

        None
    }

    pub async fn switch(&self) -> Result<(), error::Error> {
        let (switch_type, scene) = self.next_switching_scene().await;
        debug!("Next switch type: {:?}", switch_type);

        // Set the previous scene when switch_type is normal or low
        if let SwitchType::Normal | SwitchType::Low = switch_type {
            let mut state = self.state.write().await;
            state.broadcasting_software.prev_scene = scene.to_owned();
        };

        self.switch_if_necessary(&scene, switch_type).await?;

        Ok(())
    }

    pub async fn next_switching_scene(&self) -> (SwitchType, String) {
        let (switch_type, scenes) = self.next_switch().await;

        let scene = if let SwitchType::Previous = switch_type {
            let state = self.state.read().await;
            state.broadcasting_software.prev_scene.to_owned()
        } else {
            // Should be safe since previous is handled
            scenes.type_to_scene(&switch_type).unwrap()
        };

        return (switch_type, scene);
    }

    /// Returns the type and scenes of the first stream server that is not offline
    pub async fn next_switch(&self) -> (SwitchType, SwitchingScenes) {
        let state = self.state.read().await;
        let mut server = state.switcher_state.last_used_server.to_owned();

        // Get the next type and scenes
        let (mut switch_type, mut switching_scenes) = (SwitchType::Offline, None);

        let ss = &state.config.switcher;
        let triggers = &ss.triggers;

        for s in &ss.stream_servers {
            let t = s.stream_server.switch(&triggers).await;

            if t == SwitchType::Offline {
                if let Some(lus) = &state.switcher_state.last_used_server {
                    if &s.name == lus {
                        switch_type = t;

                        if let Some(d) = &s.depends_on {
                            switching_scenes = Some(d.backup_scenes.to_owned());
                        } else {
                            switching_scenes = s.override_scenes.to_owned();
                        }
                    }
                }

                continue;
            }

            server = Some(s.name.to_owned());

            if let Some(depends_on) = &s.depends_on {
                if let Some(dep) = Self::depends_on(t, &depends_on, &ss.stream_servers).await {
                    switch_type = dep.0;
                    switching_scenes = dep.1;

                    break;
                }
            }

            switch_type = t;
            switching_scenes = s.override_scenes.to_owned();
            break;
        }

        drop(state);
        let mut state = self.state.write().await;

        let switching_scenes = match switching_scenes {
            Some(scenes) => scenes,
            None => {
                // Get default scenes
                server = None;
                state.config.switcher.switching_scenes.to_owned()
            }
        };

        debug!("Last used server set to {:?}", server);
        state.switcher_state.last_used_server = server;
        (switch_type, switching_scenes)
    }

    /// Returns the backup scenes when the depended on stream is offline
    async fn depends_on(
        switch_type: SwitchType,
        depends_on: &stream_servers::DependsOn,
        stream_servers: &[stream_servers::StreamServer],
    ) -> Option<(SwitchType, Option<SwitchingScenes>)> {
        debug!("This stream server depends on: {}", depends_on.name);

        let server = match stream_servers.iter().find(|&x| x.name == depends_on.name) {
            Some(server) => server,
            None => return None,
        };

        // got the server is it online?
        if server.stream_server.bitrate().await.message.is_some() {
            debug!("The depended stream server is online");
            return None;
        }

        // it's offline

        debug!("The depended stream server is offline. Going to use the backup scenes.");

        Some((switch_type, Some(depends_on.backup_scenes.to_owned())))
    }

    // TODO
    pub async fn switch_if_necessary(
        &self,
        switch_scene: &str,
        switch_type: SwitchType,
    ) -> Result<(), error::Error> {
        debug!(
            "Switch scene: {} Switch type: {:?}",
            switch_scene, switch_type
        );

        let state = &self.state.read().await;

        if state.broadcasting_software.current_scene == switch_scene {
            return Ok(());
        }

        if !state
            .switcher_state
            .switchable_scenes
            .contains(&state.broadcasting_software.current_scene)
        {
            return Ok(());
        }

        // TODO: maybe also check if we're still on a switchable scene before switching
        // Ignore the error.. it should work at some point
        if let Err(error) = state
            .broadcasting_software
            .connection
            .as_ref()
            .ok_or(error::Error::NoSoftwareSet)?
            .switch_scene(switch_scene)
            .await
        {
            error!("Switch scene error {:?}", error);
            return Ok(());
        }

        info!("Scene switched to [{:?}] {}", switch_type, switch_scene);

        if state.broadcasting_software.is_streaming
            && state.config.switcher.auto_switch_notification
        {
            if let Some(chat) = &state.config.chat {
                let message =
                    chat::HandleMessage::AutomaticSwitchingScene(chat::AutomaticSwitchingScene {
                        platform: chat.platform.to_owned(),
                        channel: chat.username.to_owned(),
                        scene: switch_scene.to_owned(),
                        switch_type,
                    });

                let _ = self.chat_sender.send(message).await;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug)]
pub enum TriggerType {
    Low,
    Rtt,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SwitchType {
    Normal,
    Low,
    Previous,
    Offline,
}
