use std::{sync::Arc, time::Duration};

use async_recursion::async_recursion;
use async_trait::async_trait;
use futures_util::StreamExt;
use obwsv5::{
    events::Event,
    requests::{
        inputs::{self, InputId},
        scene_items::SetEnabled,
        scenes::SceneId,
    },
    responses::media_inputs::MediaState,
    Client,
};
use tokio::sync::{self, mpsc, Mutex};
use tracing::{error, info, warn, Instrument};

use crate::{
    config::{self, ObsConfig},
    error, noalbs,
    state::{self, ClientStatus},
};

use super::{
    obs::{FfmpegSource, SourceItem, VlcSource},
    BroadcastingSoftwareLogic,
};

pub struct Obsv5 {
    connection: Arc<Mutex<Option<obwsv5::Client>>>,
    connection_join: tokio::task::JoinHandle<()>,
    event_join: tokio::task::JoinHandle<()>,
}

impl Obsv5 {
    pub fn new(connection_info: config::ObsConfig, state: noalbs::UserState) -> Self {
        // OBS connection will be held in this arc mutex
        let connection = Arc::new(Mutex::new(None));

        // Will be used to receive events from OBS
        let (event_tx, event_rx) = mpsc::channel(100);

        let connection_inner = connection.clone();
        let state_inner = state.clone();
        let connection_join = tokio::spawn(async {
            let user = { state_inner.read().await.config.user.name.to_owned() };

            async move {
                let connection = InnerConnection {
                    connection_info,
                    state: state_inner,
                    connection: connection_inner,
                    event_sender: event_tx,
                };

                connection.run().await;
            }
            .instrument(tracing::info_span!("OBS", %user))
            .await
        });

        let event_join = tokio::spawn(Self::event_handler(event_rx, state));

        Self {
            connection,
            connection_join,
            event_join,
        }
    }

    async fn event_handler(mut events: mpsc::Receiver<Event>, user_state: noalbs::UserState) {
        while let Some(event) = events.recv().await {
            match event {
                Event::CurrentProgramSceneChanged { id } => {
                    let name = id.name;
                    let mut l = user_state.write().await;

                    let switchable = &l.switcher_state.switchable_scenes;
                    if switchable.contains(&name) {
                        l.broadcasting_software
                            .switch_scene_notifier()
                            .notify_waiters();
                    }

                    l.broadcasting_software.current_scene = name;
                }
                Event::StreamStateChanged { active, .. } => {
                    let mut l = user_state.write().await;

                    if active {
                        l.broadcasting_software.is_streaming = true;
                        l.broadcasting_software.last_stream_started_at = std::time::Instant::now();

                        l.broadcasting_software
                            .start_streaming_notifier()
                            .notify_waiters();

                        drop(l);

                        let ss = {
                            let read = &user_state.read().await;
                            if let Some(client) = &read.broadcasting_software.connection {
                                client.info(read).await.ok()
                            } else {
                                None
                            }
                        };

                        user_state
                            .write()
                            .await
                            .broadcasting_software
                            .initial_stream_status = ss;
                    } else {
                        l.broadcasting_software.is_streaming = false;
                        l.broadcasting_software.stream_status = None;
                        l.broadcasting_software.initial_stream_status = None;
                    }
                }
                _ => {}
            }
        }
    }

    async fn get_scenes(&self) -> Result<Vec<String>, error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        let scenes = client.scenes().list().await?;

        let mut all_scenes = Vec::new();

        for scene in scenes.scenes {
            all_scenes.push(scene.name);
        }

        Ok(all_scenes)
    }

    /// Grabs all the media sources from the current and nested scenes
    /// that are currently active.
    async fn get_media_sources(&self) -> Result<Vec<SourceItem>, error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        let mut sources: Vec<SourceItem> = Vec::new();
        let current_scene = client.scenes().current_program_scene().await?.id.name;
        get_media_sources_rec(client, current_scene, &mut Vec::new(), &mut sources, false).await;

        Ok(sources)
    }

    async fn get_sources(&self) -> Result<Vec<SourceItem>, error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        let mut sources: Vec<SourceItem> = Vec::new();
        let current_scene = client.scenes().current_program_scene().await?.id.name;
        get_sources_rec(client, current_scene, &mut Vec::new(), &mut sources, false).await;

        Ok(sources)
    }
}

#[async_recursion]
async fn get_media_sources_rec(
    client: &Client,
    scene: String,
    visited: &mut Vec<String>,
    sources: &mut Vec<SourceItem>,
    group: bool,
) {
    let items = if !group {
        client
            .scene_items()
            .list(SceneId::Name(&scene))
            .await
            .unwrap()
    } else {
        client
            .scene_items()
            .list_group(SceneId::Name(&scene))
            .await
            .unwrap()
    };
    let current_name = scene;

    for item in items {
        if let Some(ref input_kind) = item.input_kind {
            if matches!(input_kind.as_ref(), "ffmpeg_source" | "vlc_source") {
                let status = match client
                    .media_inputs()
                    .status(InputId::Name(&item.source_name))
                    .await
                {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                if matches!(
                    status.state,
                    MediaState::Playing | MediaState::Buffering | MediaState::Opening
                ) {
                    sources.push(SourceItem {
                        id: item.id,
                        scene_name: current_name.to_owned(),
                        source_name: item.source_name,
                        source_kind: input_kind.to_owned(),
                    });
                }
                continue;
            }
        }

        if matches!(
            item.source_type,
            obwsv5::responses::scene_items::SourceType::Scene
        ) && !visited.contains(&item.source_name)
        {
            visited.push(item.source_name.to_owned());

            // Should always be present because of the type check
            let group = item.is_group.unwrap();

            get_media_sources_rec(client, item.source_name, visited, sources, group).await;
        }
    }
}

#[async_recursion]
async fn get_sources_rec(
    client: &Client,
    scene: String,
    visited: &mut Vec<String>,
    sources: &mut Vec<SourceItem>,
    group: bool,
) {
    let items = if !group {
        client
            .scene_items()
            .list(SceneId::Name(&scene))
            .await
            .unwrap()
    } else {
        client
            .scene_items()
            .list_group(SceneId::Name(&scene))
            .await
            .unwrap()
    };
    let current_name = scene;

    for item in items {
        sources.push(SourceItem {
            id: item.id,
            scene_name: current_name.to_owned(),
            source_name: item.source_name.to_owned(),
            source_kind: String::new(), // Doesn't matter
        });

        if matches!(
            item.source_type,
            obwsv5::responses::scene_items::SourceType::Scene
        ) && !visited.contains(&item.source_name)
        {
            visited.push(item.source_name.to_owned());

            // Should always be present because of the type check
            let group = item.is_group.unwrap();

            get_sources_rec(client, item.source_name, visited, sources, group).await;
        }
    }
}

#[async_trait]
impl BroadcastingSoftwareLogic for Obsv5 {
    async fn switch_scene(&self, scene: &str) -> Result<String, error::Error> {
        let scenes = self.get_scenes().await?;
        let scene = scene.to_lowercase();

        let res = scenes
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let s = &s.to_lowercase();
                (i, strsim::normalized_damerau_levenshtein(&scene, s))
            })
            .min_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let scene = if let Some(s) = res {
            scenes[s.0].to_owned()
        } else {
            scene
        };

        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        client
            .scenes()
            .set_current_program_scene(SceneId::Name(&scene))
            .await?;
        Ok(scene)
    }

    async fn start_streaming(&self) -> Result<(), error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        Ok(client.streaming().start().await?)
    }

    async fn stop_streaming(&self) -> Result<(), error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        Ok(client.streaming().stop().await?)
    }

    async fn fix(&self) -> Result<(), error::Error> {
        let media_playing = self.get_media_sources().await?;

        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        for media in media_playing {
            let media_inputs = match media.source_kind.as_ref() {
                "ffmpeg_source" => {
                    let source = client
                        .inputs()
                        .settings::<FfmpegSource>(InputId::Name(&media.source_name))
                        .await?;

                    if let Some(input) = source.settings.input {
                        Vec::from([input.to_lowercase()])
                    } else {
                        continue;
                    }
                }
                "vlc_source" => client
                    .inputs()
                    .settings::<VlcSource>(InputId::Name(&media.source_name))
                    .await?
                    .settings
                    .playlist
                    .iter()
                    .map(|s| s.value.to_lowercase())
                    .collect::<Vec<String>>(),
                s => unimplemented!("Fix not implemented for {}", s),
            };

            if !media_inputs
                .iter()
                .any(|m| m.starts_with("rtmp") || m.starts_with("srt") || m.starts_with("udp"))
            {
                continue;
            }

            client
                .inputs()
                .set_settings(inputs::SetSettings {
                    input: InputId::Name(&media.source_name),
                    settings: &serde_json::json!({}),
                    overlay: None,
                })
                .await?;
        }

        Ok(())
    }

    async fn toggle_recording(&self) -> Result<(), error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        client.recording().toggle().await?;

        Ok(())
    }

    async fn is_recording(&self) -> Result<bool, error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        let status = client.recording().status().await?;
        Ok(status.active)
    }

    async fn get_media_source_status(
        &self,
        _source_name: &str,
    ) -> Result<(obws::responses::MediaState, i64), error::Error> {
        Err(error::Error::UnableInitialConnection)
    }

    async fn create_special_media_source(
        &self,
        _source_name: &str,
        _scene_name: &str,
    ) -> Result<String, error::Error> {
        Err(error::Error::UnableInitialConnection)
    }

    async fn remove_special_media_source(
        &self,
        _source_name: &str,
        _scene: &str,
    ) -> Result<(), error::Error> {
        Err(error::Error::UnableInitialConnection)
    }

    async fn current_scene(&self) -> Result<String, error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        Ok(client.scenes().current_program_scene().await?.id.name)
    }

    async fn info(
        &self,
        state: &sync::RwLockReadGuard<state::State>,
    ) -> Result<state::StreamStatus, error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        let stats = client
            .general()
            .stats()
            .await
            .map_err(error::Error::ObsV5Error)?;

        let prev_stream = client
            .streaming()
            .status()
            .await
            .map_err(error::Error::ObsV5Error)?;

        tokio::time::sleep(Duration::from_secs(2)).await;

        let stream = client
            .streaming()
            .status()
            .await
            .map_err(error::Error::ObsV5Error)?;

        let bytes_delta = (stream.bytes - prev_stream.bytes) as f64 * 8.0;
        let time_delta = stream.duration.as_seconds_f64() - prev_stream.duration.as_seconds_f64();

        let mut ss = state::StreamStatus {
            bitrate: (bytes_delta / time_delta / 1000.0) as u64,
            fps: stats.active_fps,
            num_dropped_frames: stream.skipped_frames as u64,
            num_total_frames: stream.total_frames as u64,
            output_total_frames: stats.output_total_frames as u64,
            output_skipped_frames: stats.output_skipped_frames as u64,
            render_missed_frames: stats.render_skipped_frames as u64,
            render_total_frames: stats.render_total_frames as u64,
        };

        if state.broadcasting_software.initial_stream_status.is_some() {
            ss = ss.calculate_current(
                state
                    .broadcasting_software
                    .initial_stream_status
                    .as_ref()
                    .unwrap(),
            );
        };

        Ok(ss)
    }

    async fn toggle_source(&self, source: &str) -> Result<(String, bool), error::Error> {
        let sources = self.get_sources().await?;
        let source = source.to_lowercase();

        let res = sources
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let s = &s.source_name.to_lowercase();
                (i, strsim::normalized_damerau_levenshtein(&source, s))
            })
            .min_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let source = if let Some(s) = res {
            &sources[s.0]
        } else {
            return Err(error::Error::NoSourceFound);
        };

        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        let enabled = !client
            .scene_items()
            .enabled(SceneId::Name(&source.scene_name), source.id)
            .await?;

        client
            .scene_items()
            .set_enabled(SetEnabled {
                scene: SceneId::Name(&source.scene_name),
                item_id: source.id,
                enabled,
            })
            .await?;

        Ok((source.source_name.to_owned(), enabled))
    }

    async fn set_collection_and_profile(
        &self,
        source: &config::CollectionPair,
    ) -> Result<(), error::Error> {
        let connection = self.connection.lock().await;

        let client = connection
            .as_ref()
            .ok_or(error::Error::UnableInitialConnection)?;

        client
            .scene_collections()
            .set_current(&source.collection)
            .await?;

        if !client.streaming().status().await?.active {
            client.profiles().set_current(&source.profile).await?;
        }

        Ok(())
    }
}

pub struct InnerConnection {
    connection_info: config::ObsConfig,
    state: noalbs::UserState,
    connection: Arc<Mutex<Option<obwsv5::Client>>>,
    event_sender: mpsc::Sender<Event>,
}

impl InnerConnection {
    async fn run(&self) {
        loop {
            let client = self.get_client().await;

            use obwsv5::requests::EventSubscription;
            let events = EventSubscription::SCENES | EventSubscription::OUTPUTS;
            if let Err(e) = client.reidentify(events).await {
                error!("Error reidentifying: {:?}", e)
            };

            let event_stream = client.events();

            {
                let state = &mut self.state.write().await;
                let bs = &mut state.broadcasting_software;

                if let Ok(s) = client.scenes().current_program_scene().await {
                    bs.current_scene = s.id.name;
                }

                if let Ok(s) = client.streaming().status().await {
                    bs.is_streaming = s.active;
                }

                bs.status = ClientStatus::Connected;

                let bs = &state.broadcasting_software;
                bs.connected_notifier().notify_waiters();

                if bs.is_streaming {
                    bs.start_streaming_notifier().notify_waiters();
                }

                if state
                    .switcher_state
                    .switchable_scenes
                    .contains(&bs.current_scene)
                {
                    bs.switch_scene_notifier().notify_waiters();
                }
            }

            if let Err(e) = &event_stream {
                error!("Error getting event stream: {}", e);
            }

            {
                let mut connection = self.connection.lock().await;
                *connection = Some(client);
            }

            {
                let ss = {
                    let read = &self.state.read().await;
                    let bs = &read.broadcasting_software;
                    let mut status = None;

                    if bs.is_streaming {
                        if let Some(client) = &bs.connection {
                            status = client.info(read).await.ok()
                        }
                    }

                    status
                };

                self.state
                    .write()
                    .await
                    .broadcasting_software
                    .initial_stream_status = ss;
            }

            Self::event_loop(event_stream.unwrap(), self.event_sender.clone()).await;

            warn!("Disconnected");

            {
                let state = &mut self.state.write().await;
                let bs = &mut state.broadcasting_software;
                bs.status = ClientStatus::Disconnected;
                bs.is_streaming = false;
            }
        }
    }

    /// Attempts to connect to OBS
    ///
    /// Blocks until a successful connection has been established.
    /// An exponential backoff strategy is used to keep retrying to connect.
    /// This will grow until the 5th retry failure after which the max seconds
    /// will be reached of 32 seconds.
    async fn get_client(&self) -> obwsv5::Client {
        let mut retry_grow = 1;

        loop {
            info!("Connecting");

            let ObsConfig {
                host,
                password,
                port,
                ..
            } = &self.connection_info;

            match Client::connect(host, *port, password.as_ref()).await {
                Ok(client) => {
                    info!("Connected");

                    break client;
                }
                Err(e) => {
                    warn!("Unable to connect due to: {}", e);

                    if let obwsv5::Error::Handshake(h) = e {
                        error!("{}", h);
                    }
                }
            };

            let wait = 1 << retry_grow;
            info!("trying to connect again in {} seconds", wait);
            tokio::time::sleep(Duration::from_secs(wait)).await;

            if retry_grow < 5 {
                retry_grow += 1;
            }
        }
    }

    /// Sends all received events to the MPSC
    ///
    /// Blocks until the stream gets disconnected.
    /// This most likely happens when the websocket server shuts down.
    async fn event_loop(
        events: impl futures_util::Stream<Item = Event>,
        event_sender: mpsc::Sender<Event>,
    ) {
        futures_util::pin_mut!(events);

        while let Some(event) = events.next().await {
            let _ = event_sender.send(event).await;
        }
    }
}

impl Drop for Obsv5 {
    // Abort the spawned tasks
    fn drop(&mut self) {
        self.connection_join.abort();
        self.event_join.abort();
    }
}
