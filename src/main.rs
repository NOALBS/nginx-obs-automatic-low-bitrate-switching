use anyhow::Result;
use noalbs::{
    broadcasting_software::{obs::Obs, SwitchingScenes},
    chat::twitch,
    print_logo,
    stream_servers::*,
    switcher,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    print_logo();
    alto_logger::init_alt_term_logger()?;

    let ss = SwitchingScenes::new("Scene", "Scene 2", "Brb");
    // TODO: Not hardcode obs details lol oops
    let obs = Arc::new(Obs::connect(ss).await?);

    let nginx = nginx::Nginx {
        stats_url: String::from("http://localhost/stats"),
        application: String::from("publish"),
        key: String::from("live"),
    };

    // srt://localhost:8080?mode=caller&streamid=publish/live/feed1
    let sls = sls::SrtLiveServer {
        stats_url: "http://127.0.0.1:8181/stats".to_string(),
        publisher: "publish/live/feed1".to_string(),
    };

    let _chat = twitch::Twitch {};

    let mut switcher_state = switcher::SwitcherState::default();
    switcher_state.add_stream_server(Box::new(nginx));
    switcher_state.add_stream_server(Box::new(sls));
    let switcher_state = Arc::new(Mutex::new(switcher_state));

    let switcher = noalbs::Switcher {
        broadcasting_software: obs.clone(),
        chat: None,
        state: switcher_state,
    };

    // let switcher_handler = tokio::spawn(switcher.run());
    // let _ = switcher_handler.await;

    switcher.run().await?;
    println!("Program finished");

    Ok(())
}
