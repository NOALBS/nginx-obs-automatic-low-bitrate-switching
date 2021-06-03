use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use log::info;
use noalbs::{
    chat::{
        chat_handler::ChatHandler,
        twitch::{self, Twitch},
    },
    db::{self, User},
    switcher,
};
use tokio::sync::{broadcast::Sender, RwLock};

#[tokio::main]
async fn main() -> Result<()> {
    noalbs::print_logo();
    alto_logger::init_alt_term_logger()?;

    let db_con = noalbs::db::Db::connect().await?;
    let all_clients = Arc::new(RwLock::new(HashMap::new()));
    let (tx, _) = tokio::sync::broadcast::channel(69);

    let chat_handler = Arc::new(ChatHandler::new(all_clients.clone()));
    let twitch_client = Arc::new(run_twitch_chat(
        tx.clone(),
        all_clients.clone(),
        chat_handler.clone(),
    ));

    let mut users = db_con.get_users().await?;
    info!(
        "Found {} user{}",
        users.len(),
        if users.len() > 1 { "s" } else { "" }
    );

    if users.is_empty() {
        info!("No users in the database, creating 715209");

        let id = db_con.create_user("715209").await?;

        let connection = db::Connection {
            channel: "715209".to_string(),
            platform: db::Platform::Twitch,
        };

        db_con.add_connection(id, connection).await?;
        users = db_con.get_users().await?;
    };

    load_users(
        users,
        &all_clients,
        &db_con,
        tx.clone(),
        twitch_client.clone(),
    );

    // db_con.delete_user(1).await?;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn load_users(
    users: Vec<User>,
    all_clients: &Arc<RwLock<HashMap<i64, noalbs::Noalbs>>>,
    db_con: &noalbs::db::Db,
    tx: Sender<switcher::AutomaticSwitchMessage>,
    twitch_client: Arc<Twitch>,
) {
    for user in users {
        let all_clients = all_clients.clone();
        let tx = tx.clone();
        let db_con = db_con.clone();
        let twitch_client = twitch_client.clone();
        tokio::spawn(async move {
            match db_con.load_user(&user, tx).await {
                Ok(mut noalbs_user) => {
                    for connection in noalbs_user.connections.iter() {
                        match connection.platform {
                            noalbs::db::Platform::Twitch => {
                                twitch_client.join(connection.channel.to_owned());
                            }
                            noalbs::db::Platform::Youtube => {}
                        }
                    }

                    noalbs_user.create_switcher();

                    let mut lock = all_clients.write().await;
                    lock.insert(user.id, noalbs_user);
                }
                Err(e) => {
                    eprintln!(
                        "Couldn't load OBS for user: [{}] {}\n{}",
                        user.id, user.username, e
                    );
                }
            }
        });
    }
}

fn run_twitch_chat(
    tx: Sender<switcher::AutomaticSwitchMessage>,
    db: Arc<RwLock<HashMap<i64, noalbs::Noalbs>>>,
    chat_handler: Arc<ChatHandler>,
) -> twitch::Twitch {
    let config =
        twitch_irc::ClientConfig::new_simple(twitch_irc::login::StaticLoginCredentials::new(
            "715209".to_string(),
            Some("OAUTH".to_string()),
        ));

    twitch::Twitch::run(config, tx.subscribe(), db, chat_handler)
}
