use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use noalbs::{
    broadcasting_software::obs::Obs,
    chat::{chat_handler::ChatHandler, twitch},
    db, print_logo, switcher, AutomaticSwitchMessage,
};
use tokio::sync::{broadcast::Sender, RwLock};

#[tokio::main]
async fn main() -> Result<()> {
    print_logo();
    alto_logger::init_alt_term_logger()?;

    let db_con = noalbs::db::Db::connect(false).await?;
    let all_clients = Arc::new(RwLock::new(HashMap::<i64, noalbs::Noalbs>::new()));
    let (tx, _) = tokio::sync::broadcast::channel(69);

    let chat_handler = Arc::new(ChatHandler::new(all_clients.clone()));
    let twitch_client = run_twitch_chat(tx.clone(), all_clients.clone(), chat_handler.clone());

    for user in db_con.get_users().await? {
        println!("Loaded user: [{}] {}", user.id, user.username);

        // Join chat on all services
        let connections = db_con.get_connections(user.id).await?;
        for connection in connections.iter() {
            match connection.platform {
                noalbs::db::Platform::Twitch => {
                    twitch_client.join(connection.channel.to_owned());
                }
                noalbs::db::Platform::Youtube => {}
            }
        }

        let all_clients = all_clients.clone();
        let tx = tx.clone();
        let db_con = db_con.clone();
        tokio::spawn(async move {
            // Connecting to OBS blocks until a successful connection
            // probably not what should happen
            //
            // also prevents the chat to grab the user since it won't be added
            // until load user is finished
            match load_user(&user, connections, tx, db_con).await {
                Ok(noalbs_user) => {
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

    let _ = twitch_client.reader_handle.await;
    unreachable!();
}

async fn load_user(
    user: &db::User,
    connections: Vec<db::Connection>,
    tx: Sender<AutomaticSwitchMessage>,
    db_con: db::Db,
) -> Result<noalbs::Noalbs> {
    let obs_config = db_con.get_broadcasting_software_details(user.id).await?;
    let switching_scenes = db_con.get_switching_scenes(user.id).await?;
    let broadcasting_software = Obs::connect(obs_config, switching_scenes).await;

    let mut switcher_state =
        switcher::SwitcherState::from(db_con.get_switcher_state(user.id).await?);
    switcher_state.triggers = db_con.get_triggers(user.id).await?;

    // TODO: what do i want to do with channel_admin
    let chat_state = noalbs::chat::State::from(db_con.get_chat_settings(user.id).await?);

    let mut noalbs_user = noalbs::Noalbs::new(
        user.id,
        broadcasting_software,
        switcher_state,
        chat_state,
        tx.clone(),
        connections,
    );

    // srt://localhost:8080?mode=caller&streamid=publish/live/feed1

    // TODO: Please refactor this lol
    use noalbs::db::StreamServerKind::*;
    for stream_servers in db_con.get_stream_servers(user.id).await? {
        match stream_servers.server {
            Belabox => {
                noalbs_user
                    .add_stream_server(noalbs::stream_servers::belabox::Belabox::from(
                        stream_servers,
                    ))
                    .await
            }
            Nginx => {
                noalbs_user
                    .add_stream_server(noalbs::stream_servers::nginx::Nginx::from(stream_servers))
                    .await
            }
            Nimble => {
                noalbs_user
                    .add_stream_server(noalbs::stream_servers::nimble::Nimble::from(stream_servers))
                    .await
            }
            Sls => {
                noalbs_user
                    .add_stream_server(noalbs::stream_servers::sls::SrtLiveServer::from(
                        stream_servers,
                    ))
                    .await
            }
        };
    }

    noalbs_user.create_switcher();

    Ok(noalbs_user)
}

fn run_twitch_chat(
    tx: Sender<AutomaticSwitchMessage>,
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
