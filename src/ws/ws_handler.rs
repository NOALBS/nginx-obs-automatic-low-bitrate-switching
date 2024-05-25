use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    RwLock,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::debug;

use crate::{user_manager, Noalbs};

use super::{
    requests::{Auth, SetPassword},
    responses, InternalClientToken, WsClient, WsMessage,
};

pub struct WsHandler {
    user_manager: user_manager::UserManager,
    clients: Arc<RwLock<HashMap<InternalClientToken, WsClient>>>,
}

impl WsHandler {
    pub fn new(user_manager: user_manager::UserManager) -> Self {
        Self {
            user_manager,
            clients: Default::default(),
        }
    }

    pub async fn handle(&self, rx: UnboundedReceiver<WsMessage>) {
        let rx = UnboundedReceiverStream::new(rx);

        rx.for_each(|ws_message| self.handle_request(ws_message))
            .await;
    }

    pub async fn new_client(&self, token: InternalClientToken, tx: UnboundedSender<String>) {
        debug!("New client connected [{}]", token);

        let mut clients = self.clients.write().await;

        let client = WsClient {
            token: None,
            _tx_chan: tx,
            user: None,
        };

        clients.insert(token, client);
    }

    pub async fn disconnected(&self, token: &str) {
        debug!("Client disconnected [{}]", token);

        if let Some(user) = &self.clients.read().await.get(token).unwrap().user {
            user.remove_event_sender(token).await;
        }

        let mut clients = self.clients.write().await;

        clients.remove(token);
    }

    async fn internal_is_auth(&self, token: &str) -> bool {
        let lock = self.clients.read().await;
        let client = lock.get(token);

        if let Some(c) = client {
            c.is_authenticated()
        } else {
            false
        }
    }

    async fn set_token(&self, internal_token: &str, token: String) {
        let mut lock = self.clients.write().await;
        let client = lock.get_mut(internal_token);

        if let Some(c) = client {
            c.token = Some(token);
        }
    }

    async fn set_user(&self, internal_token: &str, user: Arc<Noalbs>) {
        let mut lock = self.clients.write().await;
        let client = lock.get_mut(internal_token);

        if let Some(c) = client {
            c.user = Some(user.clone());
        }
    }

    async fn handle_request(&self, ws_message: WsMessage) {
        println!("handle request got this: {:?}", ws_message.message);

        use crate::ws::requests::Request;

        if let Request::Auth(auth) = &ws_message.message.request {
            self.handle_auth(auth, &ws_message).await;
            return;
        }

        // Only authenticated clients from this point
        if !self.internal_is_auth(&ws_message.internal_token).await {
            ws_message.reply(responses::Response::Error(
                responses::ResponseError::AuthorizationRequired,
            ));

            return;
        }

        match &ws_message.message.request {
            Request::SetPassword(s) => self.set_password(s, &ws_message).await,
            Request::Me => self.me(&ws_message).await,
            Request::Logout => self.logout(&ws_message).await,
            Request::Auth(_) => unreachable!(),
        };
    }

    async fn handle_auth(&self, auth: &Auth, ws_message: &WsMessage) {
        if self.internal_is_auth(&ws_message.internal_token).await {
            ws_message.reply(responses::Response::Error(
                responses::ResponseError::AlreadyAuthenticated,
            ));

            return;
        }

        let error_response = responses::Response::Error(responses::ResponseError::AuthFailed);

        let db = &self.user_manager.get();
        let users = db.read().await;
        let found_user = users.get(&auth.username);

        let user = match found_user {
            Some(user) => user,
            None => {
                ws_message.reply(error_response);
                return;
            }
        };

        let state = user.state.read().await;
        let config_user = &state.config.user;

        // TODO: clean this up
        let config_hash = match &config_user.password_hash {
            Some(hash) => hash,
            None => {
                // TODO: maybe just remove this none
                drop(state);
                user.add_event_sender(
                    ws_message.internal_token.clone(),
                    ws_message.tx_chan.clone(),
                )
                .await;

                self.set_user(&ws_message.internal_token, user.clone())
                    .await;

                let token = super::generate_token();
                self.set_token(&ws_message.internal_token, token.clone())
                    .await;

                ws_message.reply(responses::Response::SetPassword(
                    responses::SuccessfulLogin { token },
                ));

                return;
            }
        };

        if !super::verify(config_hash, auth.password.as_bytes()) {
            ws_message.reply(error_response);
            return;
        }

        drop(state);
        user.add_event_sender(
            ws_message.internal_token.clone(),
            ws_message.tx_chan.clone(),
        )
        .await;

        self.set_user(&ws_message.internal_token, user.clone())
            .await;

        let token = super::generate_token();
        self.set_token(&ws_message.internal_token, token.clone())
            .await;

        ws_message.reply(responses::Response::SuccessfulLogin(
            responses::SuccessfulLogin { token },
        ));
    }

    async fn set_password(&self, set_password: &SetPassword, ws_message: &WsMessage) {
        let lock = self.clients.read().await;
        let client = lock.get(&ws_message.internal_token).unwrap();

        let user = client.user.as_ref().unwrap();

        user.set_password(super::hash(set_password.password.as_bytes()))
            .await;
        let _ = user.save_config().await;

        ws_message.reply(responses::Response::UpdatedPassword);
    }

    async fn me(&self, ws_message: &WsMessage) {
        let lock = self.clients.read().await;
        let user = lock
            .get(&ws_message.internal_token)
            .unwrap()
            .user
            .as_ref()
            .unwrap();

        let state = user.state.read().await;
        let config = responses::Config::from(&state.config);

        ws_message.reply(responses::Response::Me(responses::Me { config }));
    }

    async fn logout(&self, ws_message: &WsMessage) {
        let mut lock = self.clients.write().await;
        let client = lock.get_mut(&ws_message.internal_token).unwrap();

        client
            .user
            .as_ref()
            .unwrap()
            .remove_event_sender(&ws_message.internal_token)
            .await;

        client.user = None;
        client.token = None;

        ws_message.reply(responses::Response::Logout);
    }
}
