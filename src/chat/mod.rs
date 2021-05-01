use crate::db;

pub mod chat_handler;
pub mod twitch;

pub struct State {
    pub enable_public_commands: bool,
    pub enable_mod_commands: bool,
    pub admin_users: Vec<String>,
    pub prefix: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            enable_public_commands: false,
            enable_mod_commands: false,
            admin_users: Vec::new(),
            prefix: "!".to_string(),
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
