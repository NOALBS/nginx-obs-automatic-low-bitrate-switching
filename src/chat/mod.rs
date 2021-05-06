use crate::db;
use std::collections::HashMap;

use self::chat_handler::{Command, Permission};

pub mod chat_handler;
pub mod twitch;

pub struct State {
    pub enable_public_commands: bool,
    pub enable_mod_commands: bool,
    pub admin_users: Vec<String>,
    pub prefix: String,
    pub commands_permissions: HashMap<Command, Permission>,
    pub commands_aliases: HashMap<String, Command>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            enable_public_commands: false,
            enable_mod_commands: false,
            admin_users: Vec::new(),
            prefix: "!".to_string(),
            commands_permissions: HashMap::new(),
            commands_aliases: HashMap::new(),
        }
    }
}

impl From<db::ChatSettings> for State {
    fn from(item: db::ChatSettings) -> Self {
        Self {
            enable_public_commands: item.enable_public_commands,
            enable_mod_commands: item.enable_mod_commands,
            prefix: item.prefix,
            ..Default::default()
        }
    }
}
