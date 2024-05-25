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
            let mut prev_switch_type: SwitchType = SwitchType::Offline;
            let mut same_type: u8 = 0;
            let mut same_type_seconds = 0;

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                tracing::debug!("Switcher loop");

                if let Some(notifier) = switcher.get_sleep_notifier_if_necessary().await {
                    notifier.notified().await;
                    info!("Switcher running");
                    continue;
                }

                if let Err(e) = switcher
                    .switch(
                        &mut prev_switch_type,
                        &mut same_type,
                        &mut same_type_seconds,
                    )
                    .await
                {
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

    async fn switch(
        &self,
        prev_switch_type: &mut SwitchType,
        same_type: &mut u8,
        same_type_seconds: &mut u32,
    ) -> Result<(), error::Error> {
        let state = self.state.read().await;

        let switcher_config = &state.config.switcher;
        let triggers = &switcher_config.triggers;
        let stream_servers = &switcher_config.stream_servers;
        let retry_attempts = &switcher_config.retry_attempts;
        let instant_recover = &switcher_config.instantly_switch_on_recover;

        let (mut server, mut current_switch_type) =
            Self::get_online_stream_server(stream_servers, triggers).await;

        // When stream comes back from offline, instantly switch.
        let mut force_switch = *instant_recover
            && *prev_switch_type == SwitchType::Offline
            && current_switch_type != SwitchType::Offline;

        if prev_switch_type == &current_switch_type {
            *same_type += 1;
        } else {
            debug!("Got different type, switching to that");

            *prev_switch_type = current_switch_type.to_owned();
            *same_type = 0;
            *same_type_seconds = 0;
        }

        debug!("type: {:?}, same: {:?}", current_switch_type, same_type);

        if let SwitchType::Previous = &current_switch_type {
            if let Some(s) = server {
                if let Some(last) = &state.switcher_state.last_used_server {
                    if last != &s.name {
                        current_switch_type = SwitchType::Normal;
                        force_switch = true;
                    }
                }
            }
        }

        if !(same_type == retry_attempts || force_switch) {
            return Ok(());
        }

        if state.broadcasting_software.is_streaming {
            *same_type_seconds += *same_type as u32;
            debug!("Same type seconds: {}", same_type_seconds);
        }

        if !state.broadcasting_software.is_streaming && *same_type_seconds > 0 {
            *same_type_seconds = 0;
        }

        *same_type = 0;

        if current_switch_type == SwitchType::Offline {
            // TODO: Refactor the timeout code
            if let Some(min) = &state.config.optional_options.offline_timeout {
                if *same_type_seconds >= (min * 60) {
                    let bsc = state
                        .broadcasting_software
                        .connection
                        .as_ref()
                        .ok_or(error::Error::NoSoftwareSet)?;

                    if let Err(error) = bsc.stop_streaming().await {
                        error!("Offline timeout error {:?}", error);
                        return Ok(());
                    }

                    if state.config.optional_options.record_while_streaming
                        && bsc.is_recording().await?
                    {
                        if let Err(error) = bsc.toggle_recording().await {
                            error!("Offline timeout error {:?}", error);
                            return Ok(());
                        }
                    }

                    if state.broadcasting_software.is_streaming {
                        if let Some(chat) = &state.config.chat {
                            let message =
                                chat::HandleMessage::InternalChatUpdate(chat::InternalChatUpdate {
                                    platform: chat.platform.kind(),
                                    channel: chat.username.to_owned(),
                                    kind: chat::InternalUpdate::OfflineTimeout,
                                });

                            let _ = self.chat_sender.send(message).await;
                        }
                    }
                }
            }

            if let Some(name) = &state.switcher_state.last_used_server {
                server = stream_servers.iter().find(|s| &s.name == name);
            }
        }

        let scenes = if let Some(scenes) = get_optional_scenes(server, &state).await {
            scenes
        } else {
            &switcher_config.switching_scenes
        };

        let scene = if let SwitchType::Previous = &current_switch_type {
            &state.broadcasting_software.prev_scene
        } else {
            // Should be safe since previous is handled
            scenes.type_to_scene(&current_switch_type).unwrap()
        }
        .to_owned();

        let server_name = server.map(|s| s.name.to_owned());

        drop(state);

        {
            let mut state = self.state.write().await;

            // Set the previous scene when switch_type is normal or low
            if let SwitchType::Normal | SwitchType::Low = current_switch_type {
                state.broadcasting_software.prev_scene = scene.to_owned();
            };

            if current_switch_type != SwitchType::Offline {
                debug!("Last used server set to {:?}", server_name);
                state.switcher_state.last_used_server = server_name;
            }
        }

        self.switch_if_necessary(&scene, current_switch_type)
            .await?;

        Ok(())
    }

    /// Gets the first online stream server with current status
    async fn get_online_stream_server<'a>(
        stream_servers: &'a [stream_servers::StreamServer],
        triggers: &'a Triggers,
    ) -> (Option<&'a stream_servers::StreamServer>, SwitchType) {
        for server in stream_servers {
            if !server.enabled {
                continue;
            }

            let switch_type = server.stream_server.switch(triggers).await;

            if switch_type == SwitchType::Offline {
                continue;
            }

            return (Some(server), switch_type);
        }

        (None, SwitchType::Offline)
    }

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
        let current_scene = &state.broadcasting_software.current_scene;

        if current_scene == switch_scene {
            return Ok(());
        }

        let skip = state
            .config
            .optional_scenes
            .starting
            .as_ref()
            .map_or(false, |starting_scene| {
                let switch_to_live = state
                    .config
                    .optional_options
                    .switch_from_starting_scene_to_live_scene;
                current_scene == starting_scene
                    && switch_to_live
                    && (switch_type == SwitchType::Offline)
            });

        if skip
            || !state
                .switcher_state
                .switchable_scenes
                .contains(&state.broadcasting_software.current_scene)
        {
            return Ok(());
        }

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
                        platform: chat.platform.kind(),
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

async fn get_optional_scenes<'a>(
    server: Option<&'a stream_servers::StreamServer>,
    state: &tokio::sync::RwLockReadGuard<'_, crate::state::State>,
) -> Option<&'a SwitchingScenes> {
    if let Some(depends) = &server?.depends_on {
        if !is_stream_server_online(&depends.name, state).await {
            debug!("The depended stream server is offline. Going to use the backup scenes.");
            return Some(&depends.backup_scenes);
        }
    }

    server?.override_scenes.as_ref()
}

async fn is_stream_server_online(
    server_name: &str,
    state: &tokio::sync::RwLockReadGuard<'_, crate::state::State>,
) -> bool {
    match state
        .config
        .switcher
        .stream_servers
        .iter()
        .find(|&x| x.name == server_name)
    {
        Some(server) => server.stream_server.bitrate().await.message.is_some(),
        None => false,
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

    pub fn type_to_scene(&self, s_type: &SwitchType) -> Result<&str, error::Error> {
        Ok(match s_type {
            SwitchType::Normal => &self.normal,
            SwitchType::Low => &self.low,
            SwitchType::Offline => &self.offline,
            _ => return Err(error::Error::SwitchTypeNotSupported),
        })
    }
}

#[derive(Debug)]
pub enum TriggerType {
    Low,
    Rtt,
    Offline,
    RttOffline,
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

    /// Trigger to switch to the offline scene when RTT is high
    pub rtt_offline: Option<u32>,
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
            rtt_offline: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SwitchType {
    Normal,
    Low,
    Previous,
    Offline,
}
