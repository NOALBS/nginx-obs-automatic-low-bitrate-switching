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

pub struct State {
    prev_scene: String,
    curent_scene: String,
    status: ClientStatus,
    is_streaming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientStatus {
    Connected,
    Disconnected,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Config {
    /// The hostname
    pub host: String,
    /// The password
    pub password: String,
    /// Port to connect to
    pub port: u16,
}

// TODO: Maybe remove the client Arc<Mutex<Client>>
pub struct WrappedClient {
    client: Arc<Mutex<obws::Client>>,
    connected_notify: Arc<Notify>,
}

impl WrappedClient {
    async fn run(
        event_sender: mpsc::Sender<obws::events::Event>,
        state: Arc<Mutex<State>>,
        config: Config,
    ) -> Self {
        let client = Self::get_client(&config).await;
        let wrapped_client = Self {
            client: Arc::new(Mutex::new(client)),
            connected_notify: Arc::new(Notify::new()),
        };

        tokio::spawn(Self::connection_loop(
            wrapped_client.client.clone(),
            state,
            event_sender,
            wrapped_client.connected_notify.clone(),
            config,
        ));

        wrapped_client
    }

    async fn get_client(config: &Config) -> Client {
        let mut retry_grow = 1;

        loop {
            info!("Connecting");
            if let Ok(client) = Client::connect(&config.host, config.port).await {
                info!("Connected");
                break client;
            };

            let wait = 1 << retry_grow;
            warn!("Unable to connect");
            info!("trying to connect again in {} seconds", wait);
            tokio::time::sleep(Duration::from_secs(wait)).await;

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
        config: Config,
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

            let new_client = Self::get_client(&config).await;
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
}

pub struct Obs {
    wrapped_client: WrappedClient,
    pub event_handler: tokio::task::JoinHandle<()>,
    pub switching: Arc<Mutex<SwitchingScenes>>,
    pub obs_state: Arc<Mutex<State>>,
    start_streaming_notify: Arc<Notify>,
}

impl Obs {
    pub async fn connect(config: Config, switching: SwitchingScenes) -> Self {
        let (tx, rx) = mpsc::channel(69);
        let state = State {
            prev_scene: switching.normal.to_owned(),
            curent_scene: "".to_string(),
            status: ClientStatus::Disconnected,
            is_streaming: false,
        };
        let state = Arc::new(Mutex::new(state));
        let wrapped_client = WrappedClient::run(tx, state.clone(), config).await;

        let start_streaming_notify = Arc::new(Notify::new());
        let event_handler_wants_the_state = state.clone();
        let event_handler = tokio::spawn(Obs::event_handler(
            rx,
            event_handler_wants_the_state,
            start_streaming_notify.clone(),
        ));

        Obs {
            wrapped_client,
            event_handler,
            obs_state: state,
            switching: Arc::new(Mutex::new(switching)),
            start_streaming_notify,
        }
    }

    async fn event_handler(
        mut events: mpsc::Receiver<obws::events::Event>,
        state: Arc<Mutex<State>>,
        start_streaming_notifier: Arc<Notify>,
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

                    start_streaming_notifier.notify_waiters();
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

    pub fn connected_notifier(&self) -> Arc<Notify> {
        self.wrapped_client.connected_notify.clone()
    }

    pub async fn wait_to_connect(&self) {
        self.connected_notifier().notified().await;
    }

    pub fn start_streaming_notifier(&self) -> Arc<Notify> {
        self.start_streaming_notify.clone()
    }

    /// Waits until OBS starts streaming.
    pub async fn wait_till_streaming(&self) {
        self.start_streaming_notifier().notified().await;
    }

    pub async fn can_switch(&self, scene: &str) -> bool {
        let switching = self.switching.lock().await;

        scene == switching.normal || scene == switching.low || scene == switching.offline
    }

    pub async fn is_connected(&self) -> bool {
        self.get_connection_status().await == ClientStatus::Connected
    }

    pub async fn is_streaming(&self) -> bool {
        self.obs_state.lock().await.is_streaming
    }

    pub async fn get_connection_status(&self) -> ClientStatus {
        self.obs_state.lock().await.status.to_owned()
    }

    pub async fn get_current_scene(&self) -> String {
        self.obs_state.lock().await.curent_scene.to_string()
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

    pub async fn switch_scene(&self, scene: &str) -> Result<(), Error> {
        let c = self.wrapped_client.client.lock().await;
        Ok(c.scenes().set_current_scene(scene).await?)
    }

    pub async fn get_scene_list(&self) -> Result<obws::responses::SceneList, Error> {
        let c = self.wrapped_client.client.lock().await;
        Ok(c.scenes().get_scene_list().await?)
    }

    pub async fn start_streaming(&self) -> Result<(), Error> {
        let c = self.wrapped_client.client.lock().await;
        Ok(c.streaming().start_streaming(None).await?)
    }

    pub async fn stop_streaming(&self) -> Result<(), Error> {
        let c = self.wrapped_client.client.lock().await;
        Ok(c.streaming().stop_streaming().await?)
    }

    pub async fn stream_status(&self) -> Result<obws::responses::StreamingStatus, Error> {
        let c = self.wrapped_client.client.lock().await;
        Ok(c.streaming().get_streaming_status().await?)
    }
}
