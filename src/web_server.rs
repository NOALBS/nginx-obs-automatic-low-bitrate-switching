use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use tokio::sync::mpsc::{self, UnboundedSender};
use tracing::info;
use warp::{ws::WebSocket, Filter};

use crate::{user_manager::UserManager, ws};

pub struct WebServer {
    port: u16,
    websocket_handler: Arc<ws::WsHandler>,
}

impl WebServer {
    pub fn new(port: u16, user_manager: UserManager) -> Self {
        let websocket_handler = Arc::new(ws::WsHandler::new(user_manager));

        Self {
            port,
            websocket_handler,
        }
    }

    pub async fn run(&self) {
        let (wh_sender, wh_receiver) = mpsc::unbounded_channel::<ws::WsMessage>();

        let websocket_handler = self.websocket_handler.clone();

        let ws = warp::path("ws")
            // The `ws()` filter will prepare Websocket handshake...
            .and(warp::ws())
            .and(warp::any().map(move || websocket_handler.clone()))
            .and(warp::any().map(move || wh_sender.clone()))
            .map(
                move |ws: warp::ws::Ws,
                      wh: Arc<ws::WsHandler>,
                      sender: UnboundedSender<ws::WsMessage>| {
                    ws.on_upgrade(move |socket| Self::user_connected(socket, wh, sender))
                },
            );

        let routes = ws;

        info!("Running web server on 127.0.0.1:{}", self.port);
        let serve = warp::serve(routes).run(([127, 0, 0, 1], self.port));
        let wh = self.websocket_handler.handle(wh_receiver);

        tokio::join!(serve, wh);
    }

    pub async fn user_connected(
        ws: WebSocket,
        websocket_handler: Arc<ws::WsHandler>,
        wh_sender: UnboundedSender<ws::WsMessage>,
    ) {
        let (mut client_tx, mut client_rx) = ws.split();
        let (tx, rx) = mpsc::unbounded_channel();
        let mut rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

        let internal_client_token = ws::generate_token();

        websocket_handler
            .new_client(internal_client_token.clone(), tx.clone())
            .await;

        tokio::task::spawn(async move {
            while let Some(message) = rx.next().await {
                // TODO: remove this?
                // let json = serde_json::to_string(&message).unwrap();

                client_tx
                    .send(warp::ws::Message::text(message))
                    .unwrap_or_else(|e| {
                        eprintln!("websocket send error: {}", e);
                    })
                    .await;
            }
        });

        while let Some(msg) = client_rx.next().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => {
                    println!("WS READ ERROR: {}", e);
                    break;
                }
            };

            if !msg.is_text() {
                continue;
            }

            let msg = msg.to_str().unwrap();
            let parsed = serde_json::from_str::<ws::requests::RequestMessage>(msg);

            match parsed {
                Ok(p) => {
                    let _ = wh_sender.send(ws::WsMessage {
                        internal_token: internal_client_token.clone(),
                        message: p,
                        tx_chan: tx.clone(),
                    });
                }
                Err(e) => {
                    let nonce: Option<String> = {
                        let v = serde_json::from_str::<serde_json::Value>(msg);
                        if let Ok(o) = v {
                            o["nonce"].as_str().map(String::from)
                        } else {
                            None
                        }
                    };

                    let json = serde_json::to_string(&ws::responses::ResponseMessage {
                        response: ws::responses::Response::Error(
                            ws::responses::ResponseError::Deserialize(Some(e.to_string())),
                        ),
                        nonce,
                    })
                    .unwrap();

                    let _ = tx.send(json);
                }
            }
        }

        websocket_handler.disconnected(&internal_client_token).await;
    }
}
