use chat::chat_handler;
use std::sync::Arc;
use tokio::sync::{broadcast::Sender, Mutex, RwLock};

use crate::{
    broadcasting_software::obs::Obs,
    chat, db,
    stream_servers::{self, Bsl, TriggerType},
    switcher::{self, AutomaticSwitchMessage},
    Error, Switcher,
};

pub struct Noalbs {
    user_id: i64,
    pub broadcasting_software: Arc<RwLock<Obs>>,
    pub switcher_state: Arc<Mutex<switcher::SwitcherState>>,
    pub chat_state: Arc<Mutex<chat::State>>,
    pub broadcast_sender: Sender<AutomaticSwitchMessage>,
    pub connections: Vec<db::Connection>,
    pub storage: db::Db,

    pub switcher_handler: Option<tokio::task::JoinHandle<Result<(), Error>>>,
}

impl Noalbs {
    pub fn new(
        username: i64,
        broadcasting_software: Obs,
        switcher_state: switcher::SwitcherState,
        chat_state: chat::State,
        broadcast_sender: Sender<AutomaticSwitchMessage>,
        connections: Vec<db::Connection>,
        db_con: db::Db,
    ) -> Noalbs {
        let broadcasting_software = Arc::new(RwLock::new(broadcasting_software));
        let switcher_state = Arc::new(Mutex::new(switcher_state));
        let chat_state = Arc::new(Mutex::new(chat_state));

        Self {
            user_id: username,
            broadcasting_software,
            switcher_state,
            chat_state,
            broadcast_sender,
            switcher_handler: None,
            connections,
            storage: db_con,
        }
    }

    pub async fn add_stream_server<T>(&self, server: T)
    where
        T: Bsl + 'static,
    {
        let mut state = self.switcher_state.lock().await;
        state.stream_servers.push(Box::new(server));
    }

    pub fn create_switcher(&mut self) {
        let switcher = Switcher::new(
            self.user_id.to_owned(),
            self.broadcasting_software.clone(),
            self.switcher_state.clone(),
            self.broadcast_sender.clone(),
        );

        self.switcher_handler = Some(tokio::spawn(switcher.run()));
    }

    pub fn shutdown_switcher(&mut self) {
        if let Some(handler) = &self.switcher_handler {
            handler.abort();

            // Might not need to do this?
            self.switcher_handler = None;
        }
    }
}

impl Noalbs {
    pub async fn get_trigger_by_type(&self, kind: stream_servers::TriggerType) -> Option<u32> {
        let triggers = &self.switcher_state.lock().await.triggers;
        dbg!(&triggers);

        match kind {
            TriggerType::Low => triggers.low,
            TriggerType::Rtt => triggers.rtt,
            TriggerType::Offline => triggers.offline,
        }
    }

    pub async fn update_trigger(
        &self,
        kind: stream_servers::TriggerType,
        value: u32,
    ) -> Option<u32> {
        let mut state = self.switcher_state.lock().await;
        let real_value = if value == 0 { None } else { Some(value) };

        match kind {
            TriggerType::Low => state.triggers.low = real_value,
            TriggerType::Rtt => state.triggers.rtt = real_value,
            TriggerType::Offline => state.triggers.offline = real_value,
        }

        let _ = self
            .storage
            .update_triggers(self.user_id, &state.triggers)
            .await;

        real_value
    }

    pub async fn set_bitrate_switcher_state(&self, enabled: bool) {
        // TODO: save to db

        let mut lock = self.switcher_state.lock().await;
        lock.set_bitrate_switcher_enabled(enabled);
    }

    pub async fn get_notify(&self) -> bool {
        let lock = self.switcher_state.lock().await;
        lock.auto_switch_notification
    }

    pub async fn set_notify(&self, enabled: bool) {
        // TODO: save to db

        let mut lock = self.switcher_state.lock().await;
        lock.auto_switch_notification = enabled;
    }

    pub async fn set_prefix(&self, prefix: String) {
        // TODO: save to db

        let mut lock = self.chat_state.lock().await;
        lock.prefix = prefix;
    }

    pub async fn get_autostop(&self) -> bool {
        let lock = self.chat_state.lock().await;
        lock.enable_auto_stop_stream
    }

    pub async fn set_autostop(&self, enabled: bool) {
        // TODO: save to db

        let mut lock = self.chat_state.lock().await;
        lock.enable_auto_stop_stream = enabled;
    }

    pub async fn contains_alias(&self, alias: &str) -> bool {
        let lock = self.chat_state.lock().await;
        lock.commands_aliases.contains_key(alias)
    }

    pub async fn add_alias(&self, alias: String, command: chat_handler::Command) {
        // TODO: save to db

        let mut lock = self.chat_state.lock().await;
        lock.commands_aliases.insert(alias, command);
    }

    pub async fn remove_alias(&self, alias: &str) {
        // TODO: save to db

        let mut lock = self.chat_state.lock().await;
        lock.commands_aliases.remove(alias);
    }
}
