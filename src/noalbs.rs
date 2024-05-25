use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};

use crate::{
    broadcasting_software::{obs::Obs, obs_v5::Obsv5, BroadcastingSoftwareLogic},
    chat, config, error,
    state::{self, State},
    stream_servers,
    switcher::{self, Switcher},
};

/// The state of the current user
pub type UserState = Arc<RwLock<State>>;

/// MPSC to send messages to chat
pub type ChatSender = mpsc::Sender<chat::HandleMessage>;

pub struct Noalbs {
    pub state: UserState,
    pub chat_sender: ChatSender,

    // does this really need to be an option?
    pub switcher_handler: Option<tokio::task::JoinHandle<()>>,

    /// Used to save the config
    storage: Box<dyn config::ConfigLogic>,
}

impl Noalbs {
    pub async fn new(
        storage: Box<dyn config::ConfigLogic>,
        chat_sender: ChatSender,
    ) -> Result<Self, error::Error> {
        let config = storage.load()?;
        info!("Loaded user: {}", config.user.name);

        let mut state = State {
            config,
            switcher_state: state::SwitcherState::default(),
            broadcasting_software: state::BroadcastingSoftwareState::default(),
            event_senders: Vec::new(),
        };

        state.set_all_switchable_scenes();
        state
            .config
            .switcher
            .switching_scenes
            .normal
            .clone_into(&mut state.broadcasting_software.prev_scene);

        let state = Arc::new(RwLock::new(state));

        {
            let mut w_state = state.write().await;

            let connection: Box<dyn BroadcastingSoftwareLogic> = match w_state.config.software {
                config::SoftwareConnection::ObsOld(ref obs_conf) => {
                    let obs = Obs::new(obs_conf.clone(), state.clone());
                    Box::new(obs)
                }
                config::SoftwareConnection::Obs(ref obs_conf) => {
                    let obs = Obsv5::new(obs_conf.clone(), state.clone());
                    Box::new(obs)
                }
            };

            // Do i need this option here?
            w_state.broadcasting_software.connection = Some(connection);
        }

        // Add state to any OBS stream servers
        let obs_state = state.clone();
        {
            let mut r_state = state.write().await;
            let stream_servers = &mut r_state.config.switcher.stream_servers;

            for ss in stream_servers {
                if let Some(obs) = ss
                    .stream_server
                    .as_any_mut()
                    .downcast_mut::<stream_servers::Obs>()
                {
                    obs.state = Some(obs_state.clone());
                    if let Some(scenes) = &ss.override_scenes {
                        obs.scenes = Some(scenes.to_owned());
                    }
                }
            }
        }

        let mut user = Self {
            state,
            chat_sender,
            switcher_handler: None,
            storage,
        };

        user.start_switcher().await;

        Ok(user)
    }

    pub async fn add_stream_server(&self, stream_server: stream_servers::StreamServer) {
        let mut state = self.state.write().await;
        state.config.switcher.add_stream_server(stream_server);
    }

    /// Runs a new switcher
    pub async fn start_switcher(&mut self) {
        let user = { self.state.read().await.config.user.name.to_owned() };

        let span = tracing::span!(tracing::Level::INFO, "NOALBS", %user);
        let _enter = span.enter();

        let switcher = Some(Switcher::run(Switcher {
            state: self.state.clone(),
            chat_sender: self.chat_sender.clone(),
        }));

        self.switcher_handler = switcher;
    }

    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        println!("> Stopping NOALBS {}", state.config.user.name);
        state.broadcasting_software.connection = None;

        if let Some(handler) = &self.switcher_handler {
            info!("Stopping switcher");
            handler.abort();
        }
    }

    pub async fn save_config(&self) -> Result<(), error::Error> {
        let state = self.state.read().await;
        self.storage.save(&state.config)
    }

    pub async fn contains_alias(&self, alias: &str) -> Result<bool, error::Error> {
        let state = self.state.read().await;
        let chat = &state.config.chat.as_ref().ok_or(error::Error::NoChat)?;
        let commands = &chat.commands;

        if commands.is_none() {
            return Ok(false);
        }

        let commands = commands.as_ref().unwrap();

        let contains = commands.iter().any(|(_, v)| match &v.alias {
            Some(vec_alias) => vec_alias.iter().any(|a| a == alias),
            None => false,
        });

        Ok(contains)
    }

    pub async fn add_alias(
        &self,
        alias: String,
        command: chat::Command,
    ) -> Result<(), error::Error> {
        let mut state = self.state.write().await;
        let chat = state.config.chat.as_mut().ok_or(error::Error::NoChat)?;

        let commands = chat.commands.get_or_insert(HashMap::new());
        let command = commands.entry(command).or_insert(config::CommandInfo {
            alias: Some(Vec::new()),
            ..Default::default()
        });

        if command.alias.is_none() {
            command.alias = Some(Vec::new());
        }

        command.alias.as_mut().unwrap().push(alias);

        Ok(())
    }

    pub async fn remove_alias(&self, alias: &str) -> Result<bool, error::Error> {
        let mut state = self.state.write().await;
        let chat = state.config.chat.as_mut().ok_or(error::Error::NoChat)?;

        let commands = match &mut chat.commands {
            Some(c) => c,
            None => return Ok(false),
        };

        let command = commands.iter_mut().find_map(|(_, value)| {
            if let Some(aliases) = &value.alias {
                if aliases.iter().any(|x| x == alias) {
                    return Some(value);
                }
            }

            None
        });

        let command = match command {
            Some(c) => c,
            None => return Ok(false),
        };

        let aliases = match &mut command.alias {
            Some(a) => a,
            None => return Ok(false),
        };

        if let Some(index) = aliases.iter().position(|v| *v == alias) {
            aliases.swap_remove(index);

            return Ok(true);
        }

        Ok(false)
    }

    pub async fn get_trigger_by_type(&self, kind: switcher::TriggerType) -> Option<u32> {
        let state = &self.state.read().await;
        let triggers = &state.config.switcher.triggers;

        match kind {
            switcher::TriggerType::Low => triggers.low,
            switcher::TriggerType::Rtt => triggers.rtt,
            switcher::TriggerType::Offline => triggers.offline,
            switcher::TriggerType::RttOffline => triggers.rtt_offline,
        }
    }

    pub async fn update_trigger(&self, kind: switcher::TriggerType, value: u32) -> Option<u32> {
        let mut state = self.state.write().await;
        let triggers = &mut state.config.switcher.triggers;

        let real_value = if value == 0 { None } else { Some(value) };

        match kind {
            switcher::TriggerType::Low => triggers.low = real_value,
            switcher::TriggerType::Rtt => triggers.rtt = real_value,
            switcher::TriggerType::Offline => triggers.offline = real_value,
            switcher::TriggerType::RttOffline => triggers.rtt_offline = real_value,
        }

        real_value
    }

    pub async fn get_autostop(&self) -> Result<bool, error::Error> {
        let state = &self.state.read().await;
        let chat = &state.config.chat.as_ref().ok_or(error::Error::NoChat)?;

        Ok(chat.enable_auto_stop_stream_on_host_or_raid)
    }

    pub async fn set_autostop(&self, enabled: bool) -> Result<(), error::Error> {
        let mut state = self.state.write().await;
        let chat = state.config.chat.as_mut().ok_or(error::Error::NoChat)?;

        chat.enable_auto_stop_stream_on_host_or_raid = enabled;

        Ok(())
    }

    pub async fn set_instantly_switch_on_recover(&self) -> bool {
        let mut state = self.state.write().await;
        let switcher = &mut state.config.switcher;
        let toggle = !switcher.instantly_switch_on_recover;

        switcher.instantly_switch_on_recover = toggle;

        toggle
    }

    pub async fn get_enable_mod(&self) -> Result<bool, error::Error> {
        let state = &self.state.read().await;
        let chat = &state.config.chat.as_ref().ok_or(error::Error::NoChat)?;

        Ok(chat.enable_mod_commands)
    }

    pub async fn set_enable_mod(&self, enabled: bool) -> Result<(), error::Error> {
        let mut state = self.state.write().await;
        let chat = state.config.chat.as_mut().ok_or(error::Error::NoChat)?;

        chat.enable_mod_commands = enabled;

        Ok(())
    }

    pub async fn get_enable_public(&self) -> Result<bool, error::Error> {
        let state = &self.state.read().await;
        let chat = &state.config.chat.as_ref().ok_or(error::Error::NoChat)?;

        Ok(chat.enable_public_commands)
    }

    pub async fn set_enable_public(&self, enabled: bool) -> Result<(), error::Error> {
        let mut state = self.state.write().await;
        let chat = state.config.chat.as_mut().ok_or(error::Error::NoChat)?;

        chat.enable_public_commands = enabled;

        Ok(())
    }

    pub async fn get_notify(&self) -> bool {
        let state = self.state.read().await;

        state.config.switcher.auto_switch_notification
    }

    pub async fn set_notify(&self, enabled: bool) {
        let mut state = self.state.write().await;

        state.config.switcher.auto_switch_notification = enabled;
    }

    pub async fn get_retry_attempts(&self) -> u8 {
        let state = self.state.read().await;

        state.config.switcher.retry_attempts
    }

    pub async fn set_retry_attempts(&self, value: u8) {
        let mut state = self.state.write().await;

        state.config.switcher.retry_attempts = value;
    }

    pub async fn set_prefix(&self, prefix: String) -> Result<(), error::Error> {
        let mut state = self.state.write().await;
        let chat = state.config.chat.as_mut().ok_or(error::Error::NoChat)?;

        chat.prefix = prefix;

        Ok(())
    }

    pub async fn set_bitrate_switcher_state(&self, enabled: bool) {
        let mut state = self.state.write().await;

        state.config.switcher.set_bitrate_switcher_enabled(enabled);

        if enabled {
            state
                .switcher_state
                .switcher_enabled_notifier()
                .notify_waiters();
        }
    }

    pub async fn set_password(&self, password: String) {
        let mut state = self.state.write().await;

        state.config.user.password_hash = Some(password);
    }

    pub async fn add_event_sender(&self, token: String, tx_chan: mpsc::UnboundedSender<String>) {
        let mut state = self.state.write().await;

        state
            .event_senders
            .push(state::BroadcastClient { token, tx_chan });
    }

    pub async fn chat_language(&self) -> Result<chat::ChatLanguage, error::Error> {
        let state = self.state.read().await;
        let chat = &state.config.chat.as_ref().ok_or(error::Error::NoChat)?;

        Ok(chat.language.clone())
    }

    pub async fn set_chat_language(
        &self,
        language: chat::ChatLanguage,
    ) -> Result<(), error::Error> {
        let mut state = self.state.write().await;
        let chat = state.config.chat.as_mut().ok_or(error::Error::NoChat)?;

        chat.language = language;

        Ok(())
    }

    pub async fn remove_event_sender(&self, token: &str) {
        let mut state = self.state.write().await;
        let pos = state
            .event_senders
            .iter()
            .position(|e| e.token == token)
            .unwrap();

        state.event_senders.swap_remove(pos);
    }

    pub async fn send_event<T>(&self, message: T)
    where
        T: serde::Serialize,
    {
        let state = self.state.read().await;

        for sender in &state.event_senders {
            debug!("Sending event to {}", sender.token);
            sender.send(&message);
        }
    }
}

impl Drop for Noalbs {
    // Abort the switcher spawned task to stop it
    fn drop(&mut self) {
        if let Some(handler) = &self.switcher_handler {
            handler.abort();
        }
    }
}
