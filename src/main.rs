use std::path::PathBuf;
use std::{env, sync::Arc};

use anyhow::Result;
use tokio::signal;

use noalbs::{chat::ChatPlatform, config, Noalbs};
use tracing::warn;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    noalbs::print_logo();
    let _ = print_if_new_version().await;

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "noalbs=info");
    }

    let (non_blocking_appender, _guard) = tracing_appender::non_blocking(appender());
    if cfg!(windows) {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_ansi(false)
            .with_writer(non_blocking_appender)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(non_blocking_appender)
            .init();
    }

    check_env_file();

    let user_manager = noalbs::user_manager::UserManager::new();

    // Used to send messages to the chat handler
    let (chat_tx, chat_rx) = tokio::sync::mpsc::channel(100);
    let mut chat_handler = noalbs::chat::ChatHandler::new(chat_rx, user_manager.clone());

    if env::var("CONFIG_DIR").is_ok() {
        let users = load_users_from_dir(env::var("CONFIG_DIR")?, chat_tx.clone()).await?;

        for user in users {
            user_manager.add(user?).await;
        }
    } else {
        let user = load_user_from_file("config.json".to_owned(), chat_tx.clone()).await?;
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
            .filter(|(platform, _)| platform.kind() == ChatPlatform::Twitch)
        {
            twitch.join_channel(username.to_lowercase());
        }

        chat_handler.add_chat_sender(ChatPlatform::Twitch, Arc::new(twitch));
    };

    if user_manager
        .get_all_chat()
        .await
        .iter()
        .filter(|(platform, _)| platform.kind() == ChatPlatform::Kick)
        .count()
        > 0
    {
        let kick = noalbs::chat::Kick::new(chat_tx.clone());
        for (platform, username) in user_manager
            .get_all_chat()
            .await
            .iter()
            .filter(|(platform, _)| platform.kind() == ChatPlatform::Kick)
        {
            kick.join_channel(platform.clone(), username.to_lowercase())
                .await;
        }
        chat_handler.add_chat_sender(ChatPlatform::Kick, Arc::new(kick));
    }

    tokio::task::spawn(async move {
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

pub async fn load_user_from_file<P>(
    path: P,
    broadcast_tx: noalbs::ChatSender,
) -> Result<Noalbs, noalbs::error::Error>
where
    P: Into<PathBuf>,
{
    let path = path.into();
    let file = config::File { name: path };

    Noalbs::new(Box::new(file), broadcast_tx).await
}

pub async fn load_users_from_dir<P>(
    dir: P,
    broadcast_tx: noalbs::ChatSender,
) -> Result<Vec<Result<Noalbs, noalbs::error::Error>>>
where
    P: Into<PathBuf>,
{
    let dir = dir.into();

    let noalbs_users = std::fs::read_dir(dir)?
        .filter_map(|f| f.ok())
        .map(|f| f.path())
        .filter(|e| match e.extension() {
            Some(extension) => extension == "json",
            None => false,
        })
        .map(|p| Noalbs::new(Box::new(config::File { name: p }), broadcast_tx.clone()))
        .collect::<Vec<_>>();

    let noalbs_users = futures_util::future::join_all(noalbs_users).await;

    Ok(noalbs_users)
}

async fn print_if_new_version() -> Result<(), noalbs::error::Error> {
    let url = "https://api.github.com/repos/NOALBS/nginx-obs-automatic-low-bitrate-switching/releases/latest";
    let dlu = "https://github.com/NOALBS/nginx-obs-automatic-low-bitrate-switching/releases/latest";
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "nginx-obs-automatic-low-bitrate-switching",
        )
        .send()
        .await?
        .json::<GithubApi>()
        .await?;

    if !res.tag_name.contains(noalbs::VERSION) {
        println!("NEW VERSION {} AVAILABLE", res.tag_name);
        println!("Download at {}\n", dlu);
    }

    Ok(())
}

#[derive(serde::Deserialize, Debug)]
struct GithubApi {
    tag_name: String,
}

fn check_env_file() {
    if env::var("TWITCH_BOT_USERNAME").is_err() {
        warn!("Couldn't load chat credentials from .env - continuing without connecting to chat.");
        warn!(
            "Hint: rename env.example to .env and edit it with your login information - see README"
        );
        warn!("https://github.com/NOALBS/nginx-obs-automatic-low-bitrate-switching/tree/v2#readme");
    };
}

fn appender() -> Box<dyn std::io::Write + Send + 'static> {
    if let Ok(log_dir) = env::var("LOG_DIR") {
        let file_name_prefix = if let Ok(f) = env::var("LOG_FILE_NAME") {
            f
        } else {
            "noalbs.log".to_string()
        };

        Box::new(tracing_appender::rolling::daily(log_dir, file_name_prefix))
    } else {
        Box::new(std::io::stdout())
    }
}
