use crate::{
    broadcasting_software::SwitchingScenes,
    stream_servers::{self, SwitchType},
    Error,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use log::{info, warn};
use obws::{events::EventType, Client};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex, Notify};

use super::{ClientStatus, Config, State};

// TODO: Maybe remove the client Arc<Mutex<Client>>
pub struct WrappedClient {
    client: Arc<Mutex<Option<obws::Client>>>,
    connected_notify: Arc<Notify>,
}

impl WrappedClient {
    fn run(
        event_sender: mpsc::Sender<obws::events::Event>,
        state: Arc<Mutex<State>>,
        config: Config,
    ) -> Self {
        let wrapped_client = Self {
            client: Arc::new(Mutex::new(None)),
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
        client: Arc<Mutex<Option<Client>>>,
        obs_state: Arc<Mutex<State>>,
        event_sender: mpsc::Sender<obws::events::Event>,
        connected_notifier: Arc<Notify>,
        config: Config,
    ) {
        if client.lock().await.is_none() {
            let new_client = Self::get_client(&config).await;
            *client.lock().await = Some(new_client);
        }

        // Should be safe to unwrap since it literally just connected.
        let mut event_stream = client.lock().await.as_ref().unwrap().events().unwrap();

        loop {
            {
                // TODO: possibly want to store the scene list or replace this with just a
                // current scene call
                let c = client.lock().await;
                let client_lock = c.as_ref().unwrap();
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

            *client.lock().await = Some(new_client);
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
        let wrapped_client = WrappedClient::run(tx, state.clone(), config);

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
}

#[async_trait]
impl super::BroadcastingSoftwareLogic for Obs {
    fn connected_notifier(&self) -> Arc<Notify> {
        self.wrapped_client.connected_notify.clone()
    }

    fn start_streaming_notifier(&self) -> Arc<Notify> {
        self.start_streaming_notify.clone()
    }

    fn switching_scenes(&self) -> Arc<Mutex<SwitchingScenes>> {
        self.switching.clone()
    }

    fn state(&self) -> Arc<Mutex<State>> {
        self.obs_state.clone()
    }

    async fn switch_scene(&self, scene: &str) -> Result<(), Error> {
        if let Some(c) = self.wrapped_client.client.lock().await.as_ref() {
            Ok(c.scenes().set_current_scene(scene).await?)
        } else {
            Err(Error::UnableInitialConnection)
        }
    }

    // TODO
    async fn get_scene_list(&self) -> Result<super::SceneList, Error> {
        // if let Some(c) = self.wrapped_client.client.lock().await.as_ref() {
        //     let res = c.scenes().get_scene_list().await?;
        //     let scene_list = super::SceneList {
        //         current_scene: res.current_scene,
        //         scenes: res.scenes,
        //     };

        //     Ok(scene_list)
        // } else {
        //     Err(Error::UnableInitialConnection)
        // }
        todo!()
    }

    async fn start_streaming(&self) -> Result<(), Error> {
        if let Some(c) = self.wrapped_client.client.lock().await.as_ref() {
            Ok(c.streaming().start_streaming(None).await?)
        } else {
            Err(Error::UnableInitialConnection)
        }
    }

    async fn stop_streaming(&self) -> Result<(), Error> {
        if let Some(c) = self.wrapped_client.client.lock().await.as_ref() {
            Ok(c.streaming().stop_streaming().await?)
        } else {
            Err(Error::UnableInitialConnection)
        }
    }

    async fn stream_status(&self) -> Result<super::StreamingStatus, Error> {
        if let Some(c) = self.wrapped_client.client.lock().await.as_ref() {
            let res = c.streaming().get_streaming_status().await?;
            let status = super::StreamingStatus {
                streaming: res.streaming,
                recording: res.recording,
                recording_paused: res.recording_paused,
            };

            Ok(status)
        } else {
            Err(Error::UnableInitialConnection)
        }
    }

    async fn toggle_recording(&self) -> Result<(), Error> {
        if let Some(c) = self.wrapped_client.client.lock().await.as_ref() {
            Ok(c.recording().start_stop_recording().await?)
        } else {
            Err(Error::UnableInitialConnection)
        }
    }

    async fn recording_status(&self) -> Result<super::RecordingStatus, Error> {
        if let Some(c) = self.wrapped_client.client.lock().await.as_ref() {
            let res = c.recording().get_recording_status().await?;
            let status = super::RecordingStatus {
                is_recording: res.is_recording,
                is_recording_paused: res.is_recording_paused,
            };

            Ok(status)
        } else {
            Err(Error::UnableInitialConnection)
        }
    }
}
