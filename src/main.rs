use std::path::PathBuf;
use std::{env, sync::Arc};

use anyhow::Result;
use tokio::signal;

use noalbs::{Noalbs, chat::ChatPlatform, config};
use tracing::{warn, info, error};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    noalbs::print_logo();
    let _ = print_if_new_version().await;

    if env::var("RUST_LOG").is_err() {
        unsafe { env::set_var("RUST_LOG", "noalbs=info"); }
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

    // ---- TWITCH OAUTH (IRC + HELIX). Now with support for App Access Token (badge) ----
    if let (Ok(bot_username), Ok(oauth_env)) = (env::var("TWITCH_BOT_USERNAME"), env::var("TWITCH_BOT_OAUTH")) {
        // Trim .env noise
        let oauth_raw = oauth_env.trim().to_string();

        // IRC (twitch_irc expects the token without the "oauth:" prefix). It also works if you provide it without "oauth:" in .env.
        let oauth_for_irc = oauth_raw.strip_prefix("oauth:").unwrap_or(&oauth_raw).to_string();

        // USER token (bearer) for Helix fallback
        let user_bearer = oauth_raw.strip_prefix("oauth:").unwrap_or(&oauth_raw).to_string();

        // Debug (without leaking the token)
        info!(
            "TWITCH_BOT_OAUTH loaded (has_oauth_prefix: {}, len: {})",
            oauth_raw.starts_with("oauth:"),
            oauth_raw.len()
        );

        // ---- 1) Validate USER token to obtain bot_user_id (also needed when sending with App token) ----
        #[derive(serde::Deserialize)]
        struct ValidateUser { client_id: String, user_id: String, #[serde(default)] scopes: Vec<String> }

        let http = reqwest::Client::new();
        let user_validate = http
            .get("https://id.twitch.tv/oauth2/validate")
            .header("Authorization", format!("OAuth {}", user_bearer))
            .send()
            .await;

        let (user_client_id, bot_user_id, user_scopes, user_token_ok) = match user_validate {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<ValidateUser>().await {
                    Ok(v) => (v.client_id, v.user_id, v.scopes, true),
                    Err(e) => {
                        error!("Could not parse USER /validate: {}", e);
                        ("".to_string(), "".to_string(), Vec::new(), false)
                    }
                }
            }
            Ok(resp) => {
                let status = resp.status();
                let txt = resp.text().await.unwrap_or_default();
                error!("USER token validation failed ({}): {}", status, txt);
                ("".to_string(), "".to_string(), Vec::new(), false)
            }
            Err(e) => {
                error!("USER token validation exception: {}", e);
                ("".to_string(), "".to_string(), Vec::new(), false)
            }
        };

        // ---- 2) Try to fetch an APP ACCESS TOKEN (for bot badge) if client info is present in env ----
        let app_client_id = env::var("TWITCH_APP_CLIENT_ID").ok();
        let app_client_secret = env::var("TWITCH_APP_CLIENT_SECRET").ok();

        // Only attempt if both are present.
        let (mut use_app_token, mut helix_client_id, mut helix_token) =
            (false, user_client_id.clone(), user_bearer.clone());

        if let (Some(cid), Some(cs)) = (app_client_id.clone(), app_client_secret.clone()) {
            // Chat docs require an App Access Token + chat scopes for badge.
            // https://dev.twitch.tv/docs/api/reference/#send-chat-message
            let form = [
                ("client_id", cid.as_str()),
                ("client_secret", cs.as_str()),
                ("grant_type", "client_credentials"),
                // Chat scopes for sending messages:
                ("scope", "user:read:chat user:write:chat"),
            ];
            let app_token_res = http
                .post("https://id.twitch.tv/oauth2/token")
                .form(&form)
                .send()
                .await;

            #[derive(serde::Deserialize, Default)]
            struct TokenResp { access_token: String, token_type: String, expires_in: u64 }
            let mut app_ok = false;
            let mut app_token = String::new();

            match app_token_res {
                Ok(r) if r.status().is_success() => {
                    let t = r.json::<TokenResp>().await.unwrap_or_default();
                    app_token = t.access_token;
                    app_ok = !app_token.is_empty();
                }
                Ok(r) => {
                    let status = r.status();
                    let txt = r.text().await.unwrap_or_default();
                    warn!("APP token fetch failed ({}): {}", status, txt);
                }
                Err(e) => warn!("APP token fetch exception: {}", e),
            }

            if app_ok {
                // Validate the app token (mostly to inspect scopes/validity)
                let app_val = http
                    .get("https://id.twitch.tv/oauth2/validate")
                    .header("Authorization", format!("OAuth {}", app_token))
                    .send()
                    .await;

                #[derive(serde::Deserialize, Default)]
                struct ValidateApp { client_id: String, #[serde(default)] scopes: Vec<String> }

                match app_val {
                    Ok(r) if r.status().is_success() => {
                        let v = r.json::<ValidateApp>().await.unwrap_or_default();
                        // Per docs: App token + user:write:chat on the token + (channel:bot from broadcaster OR bot is MOD)
                        let has_write = v.scopes.iter().any(|s| s == "user:write:chat");
                        if has_write {
                            use_app_token = true;
                            helix_client_id = cid;   // Client-Id must match the token's client
                            helix_token = app_token; // Use the App token for Helix
                            info!("Helix chat via APP Access Token enabled! bot badge expected when the channel grants access (mod or channel:bot).");
                        } else {
                            warn!("APP token is missing 'user:write:chat' scope; sending without badge via USER token/IRC.");
                        }
                    }
                    Ok(r) => {
                        let status = r.status();
                        let txt = r.text().await.unwrap_or_default();
                        warn!("APP /validate failed ({}): {}", status, txt);
                    }
                    Err(e) => warn!("APP /validate exception: {}", e),
                }
            }
        }

        // Mode info
        if use_app_token {
            info!("Badge mode: APP token. Reminder: the bot must be MOD in the channel OR the app must have 'channel:bot' from the broadcaster. Otherwise Twitch returns 403 and we fallback/IRC.");
        } else if user_token_ok {
            info!("Helix chat via USER token enabled (no badge per Twitch rules).");
        } else {
            warn!("USER token invalid; only IRC will work.");
        }

        // ---- 3) Look up broadcaster_id for the channels we send to ----
        let mut broadcaster_ids_map = std::collections::HashMap::new();
        // Use APP token if available, otherwise USER token
        let lookup_token = if use_app_token { &helix_token } else { &user_bearer };
        let lookup_client_id = if use_app_token { &helix_client_id } else { &user_client_id };

        if !lookup_token.is_empty() && !lookup_client_id.is_empty() {
            let twitch_users: Vec<(config::ConfigChatPlatform, String)> =
                user_manager.get_all_chat().await
                    .into_iter()
                    .filter(|(platform, _)| platform.kind() == ChatPlatform::Twitch)
                    .collect();

            let mut login_params: Vec<String> = twitch_users
                .iter()
                .map(|(_, username)| username.to_lowercase())
                .collect();

            // Include the bot user as well so we can fetch bot_user_id if user_validate failed
            if !login_params.iter().any(|u| u == &bot_username.to_lowercase()) {
                login_params.push(bot_username.to_lowercase());
            }

            if !login_params.is_empty() {
                let query = login_params
                    .iter()
                    .map(|u| format!("login={}", u))
                    .collect::<Vec<_>>()
                    .join("&");
                let url = format!("https://api.twitch.tv/helix/users?{}", query);

                match http
                    .get(&url)
                    .header("Client-Id", lookup_client_id)
                    .header("Authorization", format!("Bearer {}", lookup_token))
                    .send()
                    .await
                {
                    Ok(res) if res.status().is_success() => {
                        let data = res.json::<serde_json::Value>().await.unwrap_or_default();
                        if let Some(users) = data.get("data").and_then(|d| d.as_array()) {
                            let mut maybe_bot_id = None;
                            for user in users {
                                if let (Some(login), Some(id)) = (user.get("login"), user.get("id")) {
                                    if let (Some(login_str), Some(id_str)) = (login.as_str(), id.as_str()) {
                                        let login_lower = login_str.to_lowercase();
                                        if login_lower == bot_username.to_lowercase() {
                                            maybe_bot_id = Some(id_str.to_string());
                                        } else {
                                            broadcaster_ids_map.insert(login_lower, id_str.to_string());
                                        }
                                    }
                                }
                            }
                            // Fallback if USER /validate did not yield user_id
                            if bot_user_id.is_empty() {
                                if let Some(bid) = maybe_bot_id {
                                    info!("Bot user_id discovered via /users: {}", bid);
                                }
                            }
                        }
                    }
                    Ok(res) => {
                        let status = res.status();
                        let t = res.text().await.unwrap_or_default();
                        warn!("GET /helix/users failed ({}): {}", status, t);
                    }
                    Err(e) => warn!("GET /helix/users exception: {}", e),
                }
            }
        }

        // ---- 4) Start Twitch chat (IRC in, Helix out) ----
        let twitch = noalbs::chat::Twitch::new(
            bot_username,
            oauth_for_irc,           // IRC login
            chat_tx.clone(),
            helix_client_id,         // Client-Id for selected mode (APP or USER)
            helix_token,             // access token for selected mode
            bot_user_id,             // bot account's user_id (from USER validate or /users)
            broadcaster_ids_map,
            // helix_ok: at least one valid token
            true,
            // use app token? (controls badge + for_source_only payload)
            use_app_token,
        );

        for (_, username) in user_manager
            .get_all_chat()
            .await
            .iter()
            .filter(|(platform, _)| platform.kind() == ChatPlatform::Twitch)
        {
            twitch.join_channel(username.to_lowercase());
        }

        chat_handler.add_chat_sender(ChatPlatform::Twitch, Arc::new(twitch));
    } else {
        warn!("TWITCH_BOT_USERNAME and/or TWITCH_BOT_OAUTH missing. Twitch chat disabled.");
    }
    // ---- /TWITCH ----

    // Kick unchanged
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
            kick.join_channel(platform.clone(), username.to_lowercase()).await;
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
    use tracing::warn;
    if env::var("TWITCH_BOT_USERNAME").is_err() || env::var("TWITCH_BOT_OAUTH").is_err() {
        warn!("Missing TWITCH_BOT_USERNAME and/or TWITCH_BOT_OAUTH Twitch chat will not connect.");
    } else {
        info!("Twitch bot config: now supports APP token for bot badge.");
    }
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
