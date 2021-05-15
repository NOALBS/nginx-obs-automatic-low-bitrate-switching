use crate::db;
use std::collections::HashMap;

use self::chat_handler::{Command, Permission};

pub mod chat_handler;
pub mod twitch;

#[derive(Debug, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum ChatLanguage {
    En,
}

pub struct State {
    pub enable_public_commands: bool,
    pub enable_mod_commands: bool,
    pub enable_auto_stop_stream: bool,
    pub admin_users: Vec<String>,
    pub prefix: String,
    pub commands_permissions: HashMap<Command, Permission>,
    pub commands_aliases: HashMap<String, Command>,
    pub language: ChatLanguage,
}

impl Default for State {
    fn default() -> Self {
        Self {
            enable_public_commands: false,
            enable_mod_commands: false,
            enable_auto_stop_stream: true,
            admin_users: Vec::new(),
            prefix: "!".to_string(),
            commands_permissions: HashMap::new(),
            commands_aliases: HashMap::new(),
            language: ChatLanguage::En,
        }
    }
}

impl From<db::ChatSettings> for State {
    fn from(item: db::ChatSettings) -> Self {
        Self {
            enable_public_commands: item.enable_public_commands,
            enable_mod_commands: item.enable_mod_commands,
            enable_auto_stop_stream: item.enable_auto_stop_stream,
            prefix: item.prefix,
            ..Default::default()
        }
    }
}
