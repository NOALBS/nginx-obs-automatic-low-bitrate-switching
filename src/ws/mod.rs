use std::sync::Arc;

use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use base64::Engine;
use tokio::sync::mpsc;
use tracing::error;

use crate::Noalbs;

pub mod requests;
pub mod responses;
pub mod ws_handler;

pub use ws_handler::WsHandler;

pub type InternalClientToken = String;

pub struct WsClient {
    /// Unique token for the current client
    // TODO: Use token for login
    token: Option<String>,

    /// Channel used for sending to the websocket
    _tx_chan: mpsc::UnboundedSender<String>,

    /// When authenticated will point to the user data
    user: Option<Arc<Noalbs>>,
}

impl WsClient {
    pub fn new(tx_chan: mpsc::UnboundedSender<String>) -> Self {
        Self {
            token: None,
            _tx_chan: tx_chan,
            user: None,
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.user.is_some()
    }
}

#[derive(Clone)]
pub struct WsMessage {
    pub internal_token: InternalClientToken,
    pub message: requests::RequestMessage,
    pub tx_chan: mpsc::UnboundedSender<String>,
}

impl WsMessage {
    pub fn reply(&self, response: responses::Response) {
        let msg = responses::ResponseMessage {
            response,
            nonce: self.message.nonce.to_owned(),
        };

        let json = serde_json::to_string(&msg).unwrap();

        if let Err(e) = self.tx_chan.send(json) {
            error!("Couldn't send reply: {}", e);
        }
    }
}

pub fn generate_token() -> String {
    let mut random_token = [0u8; 32];
    OsRng.fill_bytes(&mut random_token);

    base64::engine::general_purpose::STANDARD.encode(random_token)
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
    let parsed_hash = PasswordHash::new(password_hash).unwrap();
    argon2.verify_password(password, &parsed_hash).is_ok()
}
