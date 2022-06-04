use async_trait::async_trait;
use obws::responses::MediaState;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::{Bsl, StreamServersCommands, SwitchLogic};
use crate::{
    noalbs,
    switcher::{self, SwitchType, Triggers},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obs {
    #[serde(skip_deserializing, skip_serializing)]
    pub state: Option<noalbs::UserState>,

    #[serde(skip_deserializing, skip_serializing)]
    pub scenes: Option<switcher::SwitchingScenes>,

    /// The name of the OBS media source
    pub source: String,
}

impl Obs {
    pub async fn get_stats(&self) -> Option<(MediaState, i64)> {
        let state = self.state().read().await;
        let bsc = state.broadcasting_software.connection.as_ref().unwrap();

        let current_scene = &state.broadcasting_software.current_scene;
        let offline_scene = match &self.scenes {
            Some(s) => &s.offline,
            None => &state.config.switcher.switching_scenes.offline,
        };

        let mut name = self.source.to_owned();
        if current_scene == offline_scene {
            name += "_noalbs";

            let _ = bsc
                .create_special_media_source(&self.source, offline_scene)
                .await;

            // TODO: Should the media source be removed?
            // dbg!(bsc.remove_media_source(&name.unwrap(), o).await);
        }

        let status = bsc.get_media_source_status(&name).await.ok();
        debug!("Media source status: {:?}", status);

        status
    }

    pub fn state(&self) -> &noalbs::UserState {
        self.state.as_ref().unwrap()
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Obs {
    async fn switch(&self, _: &Triggers) -> SwitchType {
        let (state, sec) = match self.get_stats().await {
            Some(stats) => stats,
            None => return SwitchType::Offline,
        };

        if matches!(state, MediaState::Playing) && sec >= 3 {
            return SwitchType::Normal;
        }

        SwitchType::Offline
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Obs {
    async fn bitrate(&self) -> super::Bitrate {
        let (state, _) = match self.get_stats().await {
            Some(stats) => stats,
            None => return super::Bitrate { message: None },
        };

        let message = format!("{:?}", state);
        super::Bitrate {
            message: Some(message),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let (state, sec) = self.get_stats().await?;

        Some(format!("{state:?}, {sec} seconds"))
    }
}

#[typetag::serde]
impl Bsl for Obs {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
