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
