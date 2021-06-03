use std::collections::HashMap;

use crate::{
    broadcasting_software::{self, obs, SwitchingScenes},
    chat::{
        self,
        chat_handler::{Command, Permission},
        ChatLanguage,
    },
    error,
    noalbs::Noalbs,
    stream_servers,
    switcher::{self, AutomaticSwitchMessage},
};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use tokio::sync::broadcast::Sender;

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

    pub async fn get_connections(&self, user_id: i64) -> Result<Vec<Connection>, error::Error> {
        Ok(
            sqlx::query_as::<_, Connection>("SELECT * FROM connection WHERE user_id = ?")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn get_broadcasting_software_details(
        &self,
        user_id: i64,
    ) -> Result<broadcasting_software::Config, error::Error> {
        Ok(sqlx::query_as::<_, broadcasting_software::Config>(
            "SELECT * FROM broadcasting_software WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn get_switching_scenes(
        &self,
        user_id: i64,
    ) -> Result<SwitchingScenes, error::Error> {
        Ok(
            sqlx::query_as::<_, SwitchingScenes>(
                "SELECT * FROM switching_scenes WHERE user_id = ?",
            )
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?,
        )
    }

    pub async fn get_switcher_state(&self, user_id: i64) -> Result<SwitcherState, error::Error> {
        Ok(
            sqlx::query_as::<_, SwitcherState>("SELECT * FROM switcher_state WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn get_triggers(
        &self,
        user_id: i64,
    ) -> Result<stream_servers::Triggers, error::Error> {
        Ok(sqlx::query_as::<_, stream_servers::Triggers>(
            "SELECT * FROM triggers WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn get_stream_servers(
        &self,
        user_id: i64,
    ) -> Result<Vec<StreamServer>, error::Error> {
        Ok(
            sqlx::query_as::<_, StreamServer>("SELECT * FROM stream_server WHERE user_id = ?")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn get_chat_settings(&self, user_id: i64) -> Result<ChatSettings, error::Error> {
        Ok(
            sqlx::query_as::<_, ChatSettings>("SELECT * FROM chat_settings WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn get_command_permissions(
        &self,
        user_id: i64,
    ) -> Result<HashMap<Command, Permission>, error::Error> {
        let mut permissions = HashMap::new();

        let command_permissions: Vec<CommandPermission> = sqlx::query_as::<_, CommandPermission>(
            "SELECT * FROM command_permission WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        for commands in command_permissions {
            permissions.insert(commands.command, commands.permission);
        }

        Ok(permissions)
    }

    pub async fn get_command_aliases(
        &self,
        user_id: i64,
    ) -> Result<HashMap<String, Command>, error::Error> {
        let mut aliases = HashMap::new();

        let command_aliases: Vec<CommandAlias> =
            sqlx::query_as::<_, CommandAlias>("SELECT * FROM command_alias WHERE user_id = ?")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?;

        for alias in command_aliases {
            aliases.insert(alias.alias, alias.command);
        }

        Ok(aliases)
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

    /// Creates an user with defaults
    pub async fn create_user(&self, username: &str) -> Result<i64, error::Error> {
        let user_id = sqlx::query!("INSERT INTO 'USER' (username) VALUES (?)", username)
            .execute(&self.pool)
            .await?
            .last_insert_rowid();

        sqlx::query!(
            r#"
            INSERT INTO broadcasting_software (user_id, host) 
            VALUES(?, ?)
            "#,
            user_id,
            "localhost",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query!("INSERT INTO chat_settings (user_id) VALUES(?);", user_id)
            .execute(&self.pool)
            .await?;

        sqlx::query!(
            r#"
            INSERT INTO stream_server
            (user_id, name, server, stats_url, application, "key")
            VALUES(?, 'N','nginx', 'http://localhost/stats', 'publish', 'live')
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;

        sqlx::query!("INSERT INTO switcher_state (user_id) VALUES(?);", user_id)
            .execute(&self.pool)
            .await?;

        sqlx::query!("INSERT INTO switching_scenes (user_id) VALUES(?);", user_id)
            .execute(&self.pool)
            .await?;

        sqlx::query!("INSERT INTO triggers (user_id) VALUES(?);", user_id)
            .execute(&self.pool)
            .await?;

        self.add_default_aliases(user_id).await?;

        Ok(user_id)
    }

    pub async fn add_default_aliases(&self, user_id: i64) -> Result<(), error::Error> {
        self.add_alias(
            user_id,
            CommandAlias {
                command: Command::Refresh,
                alias: "r".to_string(),
            },
        )
        .await?;

        self.add_alias(
            user_id,
            CommandAlias {
                command: Command::Fix,
                alias: "f".to_string(),
            },
        )
        .await?;

        self.add_alias(
            user_id,
            CommandAlias {
                command: Command::Bitrate,
                alias: "b".to_string(),
            },
        )
        .await?;

        self.add_alias(
            user_id,
            CommandAlias {
                command: Command::Refresh,
                alias: "r".to_string(),
            },
        )
        .await?;

        self.add_alias(
            user_id,
            CommandAlias {
                command: Command::Switch,
                alias: "ss".to_string(),
            },
        )
        .await?;

        Ok(())
    }

    // TODO: Check if alias already exists?
    pub async fn add_alias(
        &self,
        user_id: i64,
        command_alias: CommandAlias,
    ) -> Result<(), error::Error> {
        sqlx::query!(
            r#"
            INSERT INTO command_alias (user_id, command, alias)
            VALUES(?,?,?);"#,
            user_id,
            command_alias.command,
            command_alias.alias,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_connection(
        &self,
        user_id: i64,
        connection: Connection,
    ) -> Result<(), error::Error> {
        sqlx::query!(
            r#"
            INSERT INTO connection (user_id, channel, platform)
            VALUES(?,?,?);"#,
            user_id,
            connection.channel,
            connection.platform
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_user(&self, id: i64) -> Result<(), error::Error> {
        sqlx::query!("DELETE FROM 'user' WHERE id = ?", id,)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_switcher_state(
        &self,
        user_id: i64,
        switcher_state: &switcher::SwitcherState,
    ) -> Result<(), error::Error> {
        let req_int = switcher_state.request_interval.as_secs() as u32;
        sqlx::query!(
            r#"
            UPDATE switcher_state
            SET request_interval=?, bitrate_switcher_enabled=?,
            only_switch_when_streaming=?, auto_switch_notification=?
            WHERE user_id=?"#,
            req_int,
            switcher_state.bitrate_switcher_enabled,
            switcher_state.only_switch_when_streaming,
            switcher_state.auto_switch_notification,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_triggers(
        &self,
        user_id: i64,
        triggers: &stream_servers::Triggers,
    ) -> Result<(), error::Error> {
        sqlx::query!(
            r#"
            UPDATE triggers
            SET low=?, rtt=?, offline=?
            WHERE user_id=?"#,
            triggers.low,
            triggers.rtt,
            triggers.offline,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_chat_settings(
        &self,
        user_id: i64,
        chat_state: &chat::State,
    ) -> Result<(), error::Error> {
        sqlx::query!(
            r#"
            UPDATE chat_settings
            SET enable_public_commands=?, enable_mod_commands=?,
            enable_auto_stop_stream=?, prefix=?, language=?
            WHERE user_id=?
            "#,
            chat_state.enable_public_commands,
            chat_state.enable_mod_commands,
            chat_state.enable_auto_stop_stream,
            chat_state.prefix,
            chat_state.language,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_alias(&self, user_id: i64, alias: &str) -> Result<(), error::Error> {
        sqlx::query!(
            "DELETE FROM command_alias WHERE user_id=? AND alias=?",
            user_id,
            alias,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn load_user(
        &self,
        user: &User,
        tx: Sender<AutomaticSwitchMessage>,
    ) -> Result<Noalbs, error::Error> {
        let obs_config = self.get_broadcasting_software_details(user.id).await?;
        let switching_scenes = self.get_switching_scenes(user.id).await?;
        let broadcasting_software = obs::Obs::connect(obs_config, switching_scenes).await;

        let mut switcher_state =
            switcher::SwitcherState::from(self.get_switcher_state(user.id).await?);
        switcher_state.triggers = self.get_triggers(user.id).await?;

        // TODO: what do i want to do with channel_admin
        let connections = self.get_connections(user.id).await?;
        let mut chat_state = chat::State::from(self.get_chat_settings(user.id).await?);
        chat_state.commands_permissions = self.get_command_permissions(user.id).await?;
        chat_state.commands_aliases = self.get_command_aliases(user.id).await?;

        let noalbs_user = Noalbs::new(
            user.to_owned(),
            broadcasting_software,
            switcher_state,
            chat_state,
            tx.clone(),
            connections,
            self.clone(),
        );

        // TODO: Please refactor this
        use StreamServerKind::*;
        for stream_servers in self.get_stream_servers(user.id).await? {
            match stream_servers.server {
                Belabox => {
                    noalbs_user
                        .add_stream_server(stream_servers::belabox::Belabox::from(stream_servers))
                        .await
                }
                Nginx => {
                    noalbs_user
                        .add_stream_server(stream_servers::nginx::Nginx::from(stream_servers))
                        .await
                }
                Nimble => {
                    noalbs_user
                        .add_stream_server(stream_servers::nimble::Nimble::from(stream_servers))
                        .await
                }
                Sls => {
                    noalbs_user
                        .add_stream_server(stream_servers::sls::SrtLiveServer::from(stream_servers))
                        .await
                }
            };
        }

        Ok(noalbs_user)
    }
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
    pub enable_auto_stop_stream: bool,
    pub prefix: String,
    pub language: ChatLanguage,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CommandPermission {
    command: Command,
    permission: Permission,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CommandAlias {
    pub command: Command,
    pub alias: String,
}
