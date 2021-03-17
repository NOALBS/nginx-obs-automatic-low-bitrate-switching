use anyhow::Result;
use noalbs::{
    broadcasting_software::{obs::Obs, SwitchingScenes},
    chat::twitch,
    print_logo,
    stream_servers::*,
    switcher,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    print_logo();
    pretty_env_logger::init();

    let ss = SwitchingScenes::new("Scene", "Scene 2", "Brb");
    let obs = Arc::new(Obs::connect(ss).await?);
    let _obs_client_clone = obs.get_inner_client_clone();

    // let nginx = nginx::Nginx {
    //     stats_url: String::from("http://localhost/stats"),
    //     application: String::from("publish"),
    //     key: String::from("live"),
    // };

    // srt://localhost:8080?mode=caller&streamid=publish/live/feed1
    let sls = sls::SrtLiveServer {
        stats_url: "http://127.0.0.1:8181/stats".to_string(),
        publisher: "publish/live/feed1".to_string(),
    };

    let _chat = twitch::Twitch {};

    let triggers = Triggers {
        low: Some(800),
        rtt: None,
        offline: None,
    };

    let state = Arc::new(Mutex::new(switcher::SwitcherState {
        request_interval: Duration::from_secs(2),
        bitrate_switcher_enabled: true,
        only_switch_when_streaming: true,
        triggers,
    }));
    let _state_clone = state.clone();

    let switcher = noalbs::Switcher {
        stream_server: Box::new(sls),
        broadcasting_software: obs.clone(),
        //chat: Some(chat),
        chat: None,
        state,
    };

    //let _ = obs.switch_scene("REFRESH").await;

    let switcher_handler = tokio::spawn(switcher.run());

    let _ = switcher_handler.await;
    println!("Program finished");
    Ok(())
}
