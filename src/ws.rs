use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, trace};
use warp::ws;

use crate::user_manager;

// TODO: Rate limit on requests?

pub async fn user_connected(client: ws::WebSocket, user_manager: user_manager::UserManager) {
    let (mut client_tx, mut client_rx) = client.split();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    let mut ws_client = WsClient {
        token: generate_token(),
        tx_chan: tx,
        user: None,
    };

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            client_tx
                .send(ws::Message::text(message))
                .unwrap_or_else(|e| {
                    eprintln!("websocket send error: {}", e);
                })
                .await;
        }
    });

    while let Some(msg) = client_rx.next().await {
        match msg {
            Ok(msg) => handle_message(msg, &user_manager, &mut ws_client).await,
            Err(e) => {
                println!("WS READ ERROR: {}", e);
                break;
            }
        };
    }

    // Client disconnected remove broadcast
    if let Some(user) = ws_client.user {
        user.remove_event_sender(ws_client.token).await;
    }
}

async fn handle_message(
    message: ws::Message,
    user_manager: &user_manager::UserManager,
    client: &mut WsClient,
) {
    let message = match message.to_str() {
        Ok(m) => m,
        Err(_) => return,
    };

    trace!("Raw WS message: {}", message);

    let m: Message = match serde_json::from_str(message) {
        Ok(m) => m,
        Err(e) => {
            debug!("Error json parse: {}", e);
            client.send(Response::<()> {
                status: ResponseStatus::Error,
                error: Some(e.to_string()),
                data: None,
            });
            return;
        }
    };

    debug!("Parsed WS message: {:?}", m);

    if let Message::Auth(auth) = m {
        handle_auth(auth, &user_manager, client).await;
        return;
    }

    if client.user.is_none() {
        client.send(Response::<()> {
            status: ResponseStatus::Error,
            error: Some("Not allowed, please authenticate".to_string()),
            data: None,
        });

        return;
    }

    // All the requests the user can do while logged in
    match m {
        Message::Auth { .. } => unreachable!(),
        Message::Me => me(client).await,
        Message::SetPassword { password } => set_password(password.as_bytes(), client).await,
    }
}

pub fn hash(password: &[u8]) -> String {
    let rational_salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password, &rational_salt)
        .unwrap()
        .to_string();

    password_hash
}

pub fn verify(password_hash: &str, password: &[u8]) -> bool {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&password_hash).unwrap();
    argon2.verify_password(password, &parsed_hash).is_ok()
}

pub fn generate_token() -> String {
    let mut random_token = [0u8; 32];
    OsRng.fill_bytes(&mut random_token);

    base64::encode(random_token)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
enum Message {
    Auth(Auth),
    Me,
    SetPassword { password: String },
    // TODO: Logout
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Auth {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<T>
where
    T: Serialize,
{
    status: ResponseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

impl<T: Serialize> Response<T> {
    pub fn as_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseStatus {
    Ok,
    Error,
    AuthFailed,
    SetPassword,
}

#[derive(Debug, Serialize, Deserialize)]
struct SuccessfulLoginResponse {
    token: String,
    msg: String,
}

pub struct WsClient {
    /// Unique token for the current client
    // TODO: Use token for login
    token: String,

    /// Channel used for sending to the websocket
    tx_chan: mpsc::UnboundedSender<String>,

    /// When authenticated will point to the user data
    user: Option<std::sync::Arc<crate::Noalbs>>,
}

impl WsClient {
    fn send<T>(&self, message: Response<T>)
    where
        T: Serialize,
    {
        if self.tx_chan.send(message.as_json()).is_err() {
            // WS disconnected.. will be handled in reader
        }
    }
}

async fn handle_auth(
    user_info: Auth,
    user_manager: &user_manager::UserManager,
    client: &mut WsClient,
) {
    let error_response = Response::<()> {
        status: ResponseStatus::AuthFailed,
        error: Some("Invalid login details".to_string()),
        data: None,
    };

    let db = user_manager.get();
    let users = db.read().await;
    let found_user = users.get(&user_info.username);

    let user = match found_user {
        Some(user) => user,
        None => {
            client.send(error_response);
            return;
        }
    };

    let state = user.state.read().await;
    let config_user = &state.config.user;

    let config_hash = match &config_user.password_hash {
        Some(hash) => hash,
        None => {
            client.user = Some(user.clone());

            drop(state);
            user.add_event_sender(client.token.clone(), client.tx_chan.clone())
                .await;

            client.send(Response::<()> {
                status: ResponseStatus::SetPassword,
                error: None,
                data: None,
            });

            return;
        }
    };

    if !verify(&config_hash, user_info.password.as_bytes()) {
        client.send(error_response);
        return;
    }

    drop(state);

    client.user = Some(user.clone());
    client.send(Response {
        status: ResponseStatus::Ok,
        error: None,
        data: Some(SuccessfulLoginResponse {
            token: client.token.to_owned(),
            msg: "Succesfully logged in".to_string(),
        }),
    });

    user.add_event_sender(client.token.clone(), client.tx_chan.clone())
        .await;
}

async fn set_password(password: &[u8], client: &mut WsClient) {
    let user = client.user.as_ref().unwrap();

    user.set_password(hash(password)).await;
    user.save_config().await;

    client.send(Response {
        status: ResponseStatus::Ok,
        error: None,
        data: Some("Updated password"),
    });
}

async fn me(client: &mut WsClient) {
    let user = client.user.as_ref().unwrap();
    let state = user.state.read().await;

    let response = Response {
        status: ResponseStatus::Ok,
        error: None,
        data: Some(&state.config),
    };

    client.send(response);
}
