use crate::{
    broadcasting_software::SwitchingScenes,
    stream_servers::{self, SwitchType},
    Error,
};
use futures::{Stream, StreamExt};
use log::{info, warn};
use obws::{events::EventType, Client};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex, Notify};

pub struct Obs {
    wrapped_client: WrappedClient,
    pub event_handler: tokio::task::JoinHandle<()>,
    pub switching: Arc<Mutex<SwitchingScenes>>,
    pub obs_state: Arc<Mutex<State>>,
}

pub struct State {
    prev_scene: String,
    curent_scene: String,
    status: ClientStatus,
    is_streaming: bool,
}

// TODO: Maybe remove the client Arc<Mutex<Client>>
pub struct WrappedClient {
    client: Arc<Mutex<obws::Client>>,
    notify: Arc<Notify>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientStatus {
    Connected,
    Disconnected,
}

impl WrappedClient {
    async fn run(
        event_sender: mpsc::Sender<obws::events::Event>,
        state: Arc<Mutex<State>>,
    ) -> Self {
        let client = Self::get_client().await;
        let wrapped_client = Self {
            client: Arc::new(Mutex::new(client)),
            notify: Arc::new(Notify::new()),
        };

        tokio::spawn(Self::connection_loop(
            wrapped_client.client.clone(),
            state,
            event_sender,
            wrapped_client.notify.clone(),
        ));

        wrapped_client
    }

    fn notifier(&self) -> Arc<Notify> {
        self.notify.clone()
    }

    async fn get_client() -> Client {
        let mut retry_grow = 1;

        loop {
            info!("Connecting");
            if let Ok(client) = Client::connect("localhost", 4444).await {
                info!("Connected");
                break client;
            };

            let wait = 1 << retry_grow;
            warn!("Unable to connect");
            info!("trying to connect again in {} seconds", wait);
            std::thread::sleep(Duration::from_secs(wait));

            if retry_grow < 2 {
                retry_grow += 1;
            }
        }
    }

    async fn connection_loop(
        client: Arc<Mutex<Client>>,
        obs_state: Arc<Mutex<State>>,
        event_sender: mpsc::Sender<obws::events::Event>,
        connected_notifier: Arc<Notify>,
    ) {
        // Should be safe to unwrap since it literally just connected.
        let mut event_stream = client.lock().await.events().unwrap();

        loop {
            {
                // TODO: possibly want to store the scene list or replace this with just a
                // current scene call
                let client_lock = client.lock().await;
                let scenes = client_lock.scenes().get_scene_list().await.unwrap();
                let streaming_status = client_lock
                    .streaming()
                    .get_streaming_status()
                    .await
                    .unwrap();

                let mut state = obs_state.lock().await;
                state.curent_scene = scenes.current_scene;
                state.status = ClientStatus::Connected;
                state.is_streaming = streaming_status.streaming;

                connected_notifier.notify_waiters();
            }

            Self::event_loop(event_stream, event_sender.clone()).await;
            warn!("Disconnected");

            {
                obs_state.lock().await.status = ClientStatus::Disconnected;
            }

            let new_client = Self::get_client().await;
            event_stream = new_client.events().unwrap();

            *client.lock().await = new_client;
        }
    }

    async fn event_loop(
        events: impl Stream<Item = obws::events::Event>,
        event_sender: mpsc::Sender<obws::events::Event>,
    ) {
        futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            let _ = event_sender.send(event).await;
        }
    }

    pub async fn switch_scene(&self, scene: &str) -> Result<(), Error> {
        let c = self.client.lock().await;
        Ok(c.scenes().set_current_scene(scene).await?)
    }

    pub async fn get_scene_list(&self) -> Result<obws::responses::SceneList, Error> {
        let c = self.client.lock().await;
        Ok(c.scenes().get_scene_list().await?)
    }

    pub async fn is_streaming(&self) -> Result<bool, Error> {
        let c = self.client.lock().await;
        let st = c.streaming().get_streaming_status().await?;
        Ok(st.streaming)
    }
}

impl Obs {
    pub async fn connect(switching: SwitchingScenes) -> Result<Self, Error> {
        let (tx, rx) = mpsc::channel(69);
        let state = State {
            prev_scene: switching.normal.to_owned(),
            curent_scene: "".to_string(),
            status: ClientStatus::Disconnected,
            is_streaming: false,
        };
        let state = Arc::new(Mutex::new(state));
        let wrapped_client = WrappedClient::run(tx, state.clone()).await;

        let event_handler_wants_the_state = state.clone();
        let event_handler = tokio::spawn(Obs::event_handler(rx, event_handler_wants_the_state));

        Ok(Obs {
            wrapped_client,
            event_handler,
            obs_state: state,
            switching: Arc::new(Mutex::new(switching)),
        })
    }

    async fn event_handler(
        mut events: mpsc::Receiver<obws::events::Event>,
        state: Arc<Mutex<State>>,
    ) {
        while let Some(event) = events.recv().await {
            match event.ty {
                EventType::SwitchScenes {
                    scene_name,
                    sources: _,
                } => {
                    let mut l = state.lock().await;
                    l.curent_scene = scene_name;
                }
                EventType::StreamStarted => {
                    let mut l = state.lock().await;
                    l.is_streaming = true;
                }
                EventType::StreamStopped => {
                    let mut l = state.lock().await;
                    l.is_streaming = false;
                }
                _ => continue,
            }
        }

        //     TODO
        //     Events we are currently using
        //     this.obs.on("StreamStatus", this.setStreamStatus.bind(this));
        //     this.obs.on("ScenesChanged", this.scenesChanged.bind(this));
    }

    pub async fn switch_scene(&self, scene: &str) -> Result<(), Error> {
        self.wrapped_client.switch_scene(scene).await
    }

    pub async fn get_scene_list(&self) -> Result<obws::responses::SceneList, Error> {
        self.wrapped_client.get_scene_list().await
    }

    pub async fn is_streaming(&self) -> bool {
        self.obs_state.lock().await.is_streaming
    }

    pub async fn is_connected(&self) -> bool {
        self.get_connection_status().await == ClientStatus::Connected
    }

    pub async fn get_connection_status(&self) -> ClientStatus {
        self.obs_state.lock().await.status.to_owned()
    }

    pub async fn wait_to_connect(&self) {
        self.wrapped_client.notifier().notified().await;
    }

    pub async fn get_current_scene(&self) -> String {
        //self.curent_scene.lock().await.to_string()
        self.obs_state.lock().await.curent_scene.to_string()
    }

    pub async fn can_switch(&self, scene: &str) -> bool {
        let switching = self.switching.lock().await;

        scene == switching.normal || scene == switching.low || scene == switching.offline
    }

    pub async fn set_prev_scene(&self, scene: String) {
        let mut prev = self.obs_state.lock().await;
        prev.prev_scene = scene;
    }

    pub async fn prev_scene(&self) -> String {
        self.obs_state.lock().await.prev_scene.to_owned()
    }

    // TODO: Do i really need this?
    pub fn get_inner_client_clone(&self) -> Arc<Mutex<Client>> {
        self.wrapped_client.client.clone()
    }

    pub async fn type_to_scene(&self, s_type: &stream_servers::SwitchType) -> String {
        let switching = self.switching.lock().await;

        match s_type {
            // Safety: Should be safe to unwrap since we are handling the previous.
            SwitchType::Normal | SwitchType::Low | SwitchType::Offline => {
                switching.type_to_scene(s_type).unwrap()
            }
            SwitchType::Previous => self.prev_scene().await,
        }
    }
}
