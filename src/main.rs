use std::path::PathBuf;
use std::{env, sync::Arc};

use anyhow::Result;
use tokio::signal;

use noalbs::{chat::ChatPlatform, config, Noalbs};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    noalbs::print_logo();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "noalbs=info");
    }

    tracing_subscriber::fmt::init();

    let user_manager = noalbs::user_manager::UserManager::new();

    // Used to send messages to the chat handler
    let (chat_tx, chat_rx) = tokio::sync::mpsc::channel(100);
    let mut chat_handler = noalbs::chat::ChatHandler::new(chat_rx, user_manager.clone());

    if env::var("CONFIG_DIR").is_ok() {
        let users = load_users_from_dir(env::var("CONFIG_DIR")?, chat_tx.clone()).await?;

        for user in users {
            user_manager.add(user).await;
        }
    } else {
        let user = load_user_from_file("config.json".to_owned(), chat_tx.clone()).await;
        user_manager.add(user).await;
    }

    if env::var("TWITCH_BOT_USERNAME").is_ok() {
        let bot_username = env::var("TWITCH_BOT_USERNAME")?;
        let oauth = env::var("TWITCH_BOT_OAUTH")?;

        let twitch = noalbs::chat::Twitch::new(bot_username, oauth, chat_tx.clone());

        for (_, username) in user_manager
            .get_all_chat()
            .await
            .iter()
            .filter(|(platform, _)| platform == &ChatPlatform::Twitch)
        {
            twitch.join_channel(username.to_lowercase());
        }

        chat_handler.add_chat_sender(ChatPlatform::Twitch, Arc::new(twitch));
    };

    let _ = tokio::task::spawn(async move {
        chat_handler.handle_messages().await;
    });

    if env::var("API_PORT").is_ok() {
        let port: u16 = env::var("API_PORT")?.parse()?;
        let webserver = noalbs::web_server::WebServer::new(port, user_manager.clone());
        webserver.run().await;
    }

    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }

    Ok(())
}

pub async fn load_user_from_file<P>(path: P, broadcast_tx: noalbs::ChatSender) -> Noalbs
where
    P: Into<PathBuf>,
{
    let path = path.into();
    let file = config::File { name: path };

    Noalbs::new(Box::new(file), broadcast_tx).await
}

pub async fn load_users_from_dir<P>(dir: P, broadcast_tx: noalbs::ChatSender) -> Result<Vec<Noalbs>>
where
    P: Into<PathBuf>,
{
    let dir = dir.into();

    let noalbs_users = std::fs::read_dir(dir)?
        .filter_map(|f| f.ok())
        .map(|f| f.path())
        .filter(|e| e.extension().unwrap() == "json")
        .map(|p| Noalbs::new(Box::new(config::File { name: p }), broadcast_tx.clone()))
        .collect::<Vec<_>>();

    let noalbs_users = futures_util::future::join_all(noalbs_users).await;

    Ok(noalbs_users)
}
