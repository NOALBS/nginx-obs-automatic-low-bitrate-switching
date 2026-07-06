use std::path::PathBuf;
use std::{
    env, fs,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use tokio::signal;

use noalbs::{Noalbs, chat::ChatPlatform, config};
use tracing::warn;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const DEFAULT_LOG_DIR: &str = "logs";
const LOG_DIR_ENV: &str = "LOG_DIR";
const LOG_FILE_NAME_ENV: &str = "LOG_FILE_NAME";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    noalbs::print_logo();
    let _ = print_if_new_version().await;

    if env::var("RUST_LOG").is_err() {
        // TODO: set_var is now unsafe. Check if it's safe to use.
        unsafe {
            env::set_var("RUST_LOG", "noalbs=info");
        }
    }

    let _guard = setup_logging(should_log_to_file());

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
        warn!("Hint: edit .env it with your login information - see README");
        warn!("https://github.com/NOALBS/nginx-obs-automatic-low-bitrate-switching/tree/v2#readme");
    };
}

/// Sets up logging to stdout plus a rotating-per-run log file. If
/// `log_to_file` is false (see `config.json`'s `logToFile` field), or file
/// logging can't be set up (e.g. the log directory isn't writable), this
/// falls back to stdout-only logging instead of failing to start -- a
/// logging problem should never prevent NOALBS from running.
fn setup_logging(log_to_file: bool) -> WorkerGuard {
    if !log_to_file {
        return setup_stdout_only_logging();
    }

    match setup_file_and_stdout_logging() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!(
                "warning: failed to set up file logging ({err}), continuing with stdout logging only"
            );
            setup_stdout_only_logging()
        }
    }
}

/// Peeks at the config file(s) to see whether file logging has been
/// disabled, without doing a full config load (which happens later, and
/// asynchronously). Logging is set up once for the whole process, so in
/// `CONFIG_DIR` (multi-user) mode, file logging is disabled if *any*
/// config explicitly sets `logToFile` to `false`. Defaults to `true` if
/// the field is missing or the file can't be read/parsed yet -- config
/// errors are surfaced properly later during the real load.
fn should_log_to_file() -> bool {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct LogToFileOnly {
        #[serde(default = "default_log_to_file")]
        log_to_file: bool,
    }

    fn default_log_to_file() -> bool {
        true
    }

    fn wants_file_logging(path: &std::path::Path) -> bool {
        fs::read_to_string(path)
            .ok()
            .and_then(|contents| serde_json::from_str::<LogToFileOnly>(&contents).ok())
            .map(|c| c.log_to_file)
            .unwrap_or(true)
    }

    match env::var("CONFIG_DIR") {
        Ok(dir) => match fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
                .all(|p| wants_file_logging(&p)),
            Err(_) => true,
        },
        Err(_) => wants_file_logging(std::path::Path::new("config.json")),
    }
}

fn setup_file_and_stdout_logging() -> Result<WorkerGuard> {
    let log_dir = env::var(LOG_DIR_ENV).unwrap_or_else(|_| DEFAULT_LOG_DIR.to_string());
    fs::create_dir_all(&log_dir)?;

    let file_name = log_file_name()?;
    let log_file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(PathBuf::from(log_dir).join(file_name))?;
    let (file_writer, guard) = tracing_appender::non_blocking(log_file);
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_ansi(!cfg!(windows))
        .with_writer(std::io::stdout);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(file_writer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}

fn setup_stdout_only_logging() -> WorkerGuard {
    let (stdout_writer, guard) = tracing_appender::non_blocking(std::io::stdout());
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(!cfg!(windows))
                .with_writer(stdout_writer),
        )
        .init();
    guard
}

fn log_file_name() -> Result<String> {
    if let Ok(file_name) = env::var(LOG_FILE_NAME_ENV) {
        return Ok(file_name);
    }

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(format!("noalbs-{}.log", timestamp))
}
