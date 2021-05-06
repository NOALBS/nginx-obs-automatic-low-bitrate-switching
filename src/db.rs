use std::collections::HashMap;

use crate::{
    broadcasting_software::{obs, SwitchingScenes},
    chat::chat_handler::{Command, Permission},
    error, stream_servers, ChatLanguage,
};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

const DB_NAME: &str = "sqlite:database.db?mode=rwc";

#[derive(Clone)]
pub struct Db {
    pool: Pool<Sqlite>,
}

impl Db {
    pub async fn connect() -> Result<Self, error::Error> {
        let pool = SqlitePoolOptions::new().connect(DB_NAME).await?;

        if let Err(e) = sqlx::migrate!().run(&pool).await {
            println!("Couldn't run migrations: {}", e);
        }

        Ok(Self { pool })
    }

    pub async fn get_users(&self) -> Result<Vec<User>, error::Error> {
        Ok(sqlx::query_as::<_, User>("SELECT * FROM user")
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn get_connections(&self, id: i64) -> Result<Vec<Connection>, error::Error> {
        Ok(
            sqlx::query_as::<_, Connection>("SELECT * FROM connection WHERE user_id = ?")
                .bind(id)
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn get_broadcasting_software_details(
        &self,
        id: i64,
    ) -> Result<obs::Config, error::Error> {
        Ok(sqlx::query_as::<_, obs::Config>(
            "SELECT * FROM broadcasting_software WHERE user_id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn get_switching_scenes(&self, id: i64) -> Result<SwitchingScenes, error::Error> {
        Ok(
            sqlx::query_as::<_, SwitchingScenes>(
                "SELECT * FROM switching_scenes WHERE user_id = ?",
            )
            .bind(id)
            .fetch_one(&self.pool)
            .await?,
        )
    }

    pub async fn get_switcher_state(&self, id: i64) -> Result<SwitcherState, error::Error> {
        Ok(
            sqlx::query_as::<_, SwitcherState>("SELECT * FROM switcher_state WHERE user_id = ?")
                .bind(id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn get_triggers(&self, id: i64) -> Result<stream_servers::Triggers, error::Error> {
        Ok(sqlx::query_as::<_, stream_servers::Triggers>(
            "SELECT * FROM triggers WHERE user_id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn get_stream_servers(&self, id: i64) -> Result<Vec<StreamServer>, error::Error> {
        Ok(
            sqlx::query_as::<_, StreamServer>("SELECT * FROM stream_server WHERE user_id = ?")
                .bind(id)
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn get_chat_settings(&self, id: i64) -> Result<ChatSettings, error::Error> {
        Ok(
            sqlx::query_as::<_, ChatSettings>("SELECT * FROM chat_settings WHERE user_id = ?")
                .bind(id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn get_command_permissions(
        &self,
        id: i64,
    ) -> Result<HashMap<Command, Permission>, error::Error> {
        let mut permissions = HashMap::new();

        let command_permissions: Vec<CommandPermission> = sqlx::query_as::<_, CommandPermission>(
            "SELECT * FROM command_permission WHERE user_id = ?",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        for commands in command_permissions {
            permissions.insert(commands.command, commands.permission);
        }

        Ok(permissions)
    }

    // pub async fn get_everything(&self) -> Result<(), error::Error> {
    //     Ok(
    //         sqlx::query_as::<_, ???>("
    //              SELECT * FROM 'user' u
    //              LEFT JOIN 'connection' c ON c.user_id = u.id
    //              LEFT JOIN broadcasting_software bs ON bs.user_id = u.id
    //              LEFT JOIN chat_settings cs ON cs.user_id = u.id
    //              LEFT JOIN stream_server ss ON ss.user_id = u.id
    //              LEFT JOIN switcher_state ss2 ON ss2.user_id = u.id
    //              LEFT JOIN switching_scenes ss3 ON ss3.user_id = u.id
    //              LEFT JOIN triggers t ON t.user_id = u.id
    //                                  ")
    //             .bind(id)
    //             .fetch_all(&self.pool)
    //             .await?,
    //     )
    // }
}

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Connection {
    pub channel: String,
    pub platform: Platform,
}

#[derive(Debug, sqlx::Type, PartialEq)]
#[sqlx(rename_all = "lowercase")]
pub enum Platform {
    Twitch,
    Youtube,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SwitcherState {
    pub request_interval: i64,
    pub bitrate_switcher_enabled: bool,
    pub only_switch_when_streaming: bool,
    pub auto_switch_notification: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct StreamServer {
    pub server: StreamServerKind,
    pub name: String,
    pub stats_url: String,
    pub application: String,
    pub key: String,
    pub udp_listener_id: String,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum StreamServerKind {
    Belabox,
    Nginx,
    Nimble,
    Sls,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ChatSettings {
    pub enable_public_commands: bool,
    pub enable_mod_commands: bool,
    pub prefix: String,
    pub language: ChatLanguage,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CommandPermission {
    command: Command,
    permission: Permission,
}
