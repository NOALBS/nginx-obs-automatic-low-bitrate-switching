use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{chat, Noalbs};

type User = Arc<RwLock<HashMap<String, Arc<Noalbs>>>>;

#[derive(Clone)]
pub struct UserManager {
    users: User,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self) -> User {
        self.users.clone()
    }

    pub async fn add(&self, user: Noalbs) {
        let lock = &mut self.users.write().await;

        let state = user.state.read().await;
        let key = state.config.user.name.clone();
        drop(state);

        lock.insert(key, Arc::new(user));
    }

    /// Returns the platform and username
    pub async fn get_all_chat(&self) -> Vec<(chat::ChatPlatform, String)> {
        let mut all_chat = Vec::new();

        let lock = self.users.read().await;
        for val in (*lock).values() {
            let state = val.state.read().await;
            if let Some(chat) = &state.config.chat {
                all_chat.push((chat.platform.to_owned(), chat.username.to_owned()));
            }
        }

        all_chat
    }

    // TODO: Probably don't want this???
    pub async fn get_user_by_chat_platform(
        &self,
        username: &str,
        platform: &chat::ChatPlatform,
    ) -> Option<Arc<Noalbs>> {
        let lock = self.users.read().await;

        for value in (*lock).values() {
            let state = &value.state.read().await;

            if let Some(chat) = &state.config.chat {
                if chat.username == username && &chat.platform == platform {
                    return Some(value.clone());
                }
            }
        }

        None
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}
