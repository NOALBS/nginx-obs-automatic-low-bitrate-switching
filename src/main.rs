use std::{env, sync::Arc};

use tokio::signal;

use noalbs::{chat::ChatPlatform, config, Noalbs};

#[tokio::main]
async fn main() {
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

    let user = load_user_from_file("config.json".to_owned(), chat_tx.clone()).await;
    user_manager.add(user).await;

    if env::var("TWITCH_BOT_USERNAME").is_ok() {
        let bot_username = env::var("TWITCH_BOT_USERNAME").unwrap();
        let oauth = env::var("TWITCH_BOT_OAUTH").unwrap();

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
        noalbs::api::run(user_manager.clone()).await;
    }

    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }
}

pub async fn load_user_from_file(name: String, broadcast_tx: noalbs::ChatSender) -> Noalbs {
    let file = config::File { name };

    Noalbs::new(Box::new(file), broadcast_tx).await
}
