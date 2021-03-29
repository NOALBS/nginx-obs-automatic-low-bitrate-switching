use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use noalbs::{
    broadcasting_software::{
        obs::{self, Obs},
        SwitchingScenes,
    },
    chat::twitch,
    print_logo,
    stream_servers::*,
    switcher, AutomaticSwitchMessage,
};
use tokio::sync::{broadcast::Sender, RwLock};

#[tokio::main]
async fn main() -> Result<()> {
    print_logo();
    alto_logger::init_alt_term_logger()?;

    // move this into lib?
    // TODO: How to do this better?
    let db = Arc::new(RwLock::new(HashMap::new()));
    let (tx, _) = tokio::sync::broadcast::channel(69);

    let chat_handler = Arc::new(noalbs::chat::chat_handler::ChatHandler::new(db.clone()));
    let twitch_client = run_twitch_chat(tx.clone(), db.clone(), chat_handler.clone());

    let _ = create_user("715209".into(), &twitch_client, tx.clone(), db.clone()).await;

    let _ = twitch_client.reader_handle.await;
    unreachable!();
}

// Just for testing
async fn create_user(
    username: String,
    twitch_client: &twitch::Twitch,
    tx: Sender<AutomaticSwitchMessage>,
    db: Arc<RwLock<HashMap<String, noalbs::Noalbs>>>,
) -> Result<()> {
    let chat_state = noalbs::chat::State::default();

    let obs_config = obs::Config {
        host: "localhost".to_string(),
        port: 4444,
    };
    let switching_scenes = SwitchingScenes::new("Scene", "Scene 2", "Brb");
    let broadcasting_software = Obs::connect(obs_config, switching_scenes).await?;
    let switcher_state = switcher::SwitcherState::default();

    let mut noalbs_user = noalbs::Noalbs::new(
        username.to_owned(),
        broadcasting_software,
        switcher_state,
        chat_state,
        tx.clone(),
    );

    noalbs_user
        .add_stream_server(nginx::Nginx {
            stats_url: String::from("http://localhost/stats"),
            application: String::from("publish"),
            key: String::from("live"),
        })
        .await;

    // srt://localhost:8080?mode=caller&streamid=publish/live/feed1
    noalbs_user
        .add_stream_server(sls::SrtLiveServer {
            stats_url: "http://127.0.0.1:8181/stats".to_string(),
            publisher: "publish/live/feed1".to_string(),
        })
        .await;

    noalbs_user.create_switcher();

    {
        let mut lock = db.write().await;
        lock.insert(username.to_owned(), noalbs_user);
    }

    twitch_client.join(&username);

    Ok(())
}

fn run_twitch_chat(
    tx: Sender<AutomaticSwitchMessage>,
    db: Arc<RwLock<HashMap<String, noalbs::Noalbs>>>,
    chat_handler: Arc<noalbs::chat::chat_handler::ChatHandler>,
) -> twitch::Twitch {
    let config =
        twitch_irc::ClientConfig::new_simple(twitch_irc::login::StaticLoginCredentials::new(
            "715209".to_string(),
            Some("OAUTH".to_string()),
        ));

    twitch::Twitch::run(config, tx.subscribe(), db, chat_handler)
}
