use anyhow::Result;
use noalbs::{
    broadcasting_software::{
        obs::{self, Obs},
        SwitchingScenes,
    },
    chat::twitch,
    print_logo,
    stream_servers::*,
    switcher, BroadcastMessage,
};
use tokio::sync::broadcast::Sender;

#[tokio::main]
async fn main() -> Result<()> {
    print_logo();
    alto_logger::init_alt_term_logger()?;

    let (tx, _rx) = tokio::sync::broadcast::channel(69);

    let twitch_client = run_twitch_chat(tx.clone());
    twitch_client.join("715209");

    // Now user:
    let obs_config = obs::Config {
        host: "localhost".to_string(),
        port: 4444,
    };
    let switching_scenes = SwitchingScenes::new("Scene", "Scene 2", "Brb");
    let broadcasting_software = Obs::connect(obs_config, switching_scenes).await?;
    let switcher_state = switcher::SwitcherState::default();

    let mut _user = noalbs::Noalbs::new(
        "715209".to_string(),
        broadcasting_software,
        switcher_state,
        tx.clone(),
    );

    _user
        .add_stream_server(nginx::Nginx {
            stats_url: String::from("http://localhost/stats"),
            application: String::from("publish"),
            key: String::from("live"),
        })
        .await;

    // srt://localhost:8080?mode=caller&streamid=publish/live/feed1
    _user
        .add_stream_server(sls::SrtLiveServer {
            stats_url: "http://127.0.0.1:8181/stats".to_string(),
            publisher: "publish/live/feed1".to_string(),
        })
        .await;

    _user.create_switcher();

    let _ = _user.switcher_handler.unwrap().await;
    let _ = twitch_client.reader_handle.await;
    println!("Program finished");
    Ok(())
}

fn run_twitch_chat(tx: Sender<BroadcastMessage>) -> twitch::Twitch {
    let config =
        twitch_irc::ClientConfig::new_simple(twitch_irc::login::StaticLoginCredentials::new(
            "715209".to_string(),
            Some("OAUTH".to_string()),
        ));

    twitch::Twitch::run(config, tx.subscribe())
}
