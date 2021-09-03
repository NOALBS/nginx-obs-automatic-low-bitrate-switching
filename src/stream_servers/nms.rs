use async_trait::async_trait;
use log::{error, trace};
use serde::{Deserialize, Serialize};
use tokio::time::{self, Duration};
use tracing::debug;

use super::{Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Stat {
    pub is_live: bool,
    pub viewers: u64,
    pub duration: u64,
    pub bitrate: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Auth {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeMediaServer {
    /// Url to the NGINX stats page
    pub stats_url: String,

    /// Stream application
    pub application: String,

    /// Stream key
    pub key: String,

    /// Max attempts to poll the bitrate every second on low bitrate.
    /// This will be used to make sure the stream is actually in a low
    /// bitrate state
    #[serde(default = "super::default_max_low_retry")]
    pub max_low_retry: u8,

    pub auth: Option<Auth>,
}

impl NodeMediaServer {
    pub async fn get_stats(&self) -> Option<Stat> {
        let url = format!("{}/{}/{}", &self.stats_url, &self.application, &self.key);

        let client = reqwest::Client::new();
        let mut request = client.get(url);

        if let Some(auth) = &self.auth {
            request = request.basic_auth(&auth.username, Some(&auth.password));
        }

        let res = match request.send().await {
            Ok(res) => res,
            Err(_) => {
                error!("Stats page ({}) is unreachable", self.stats_url);
                return None;
            }
        };

        if res.status() != reqwest::StatusCode::OK {
            error!("Error accessing stats page ({})", self.stats_url);
            return None;
        }

        let text = res.text().await.ok()?;
        let stream: Stat = serde_json::from_str(&text).ok()?;

        trace!("{:#?}", stream);
        Some(stream)
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for NodeMediaServer {
    /// Which scene to switch to
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let stats = match self.get_stats().await {
            Some(b) => b,
            None => return SwitchType::Offline,
        };

        if !stats.is_live {
            return SwitchType::Offline;
        }

        if let Some(offline) = triggers.offline {
            if stats.bitrate > 0 && stats.bitrate <= offline.into() {
                return SwitchType::Offline;
            }
        }

        if stats.bitrate == 0 {
            return SwitchType::Previous;
        }

        if let Some(low) = triggers.low {
            if stats.bitrate <= low.into() {
                let mut amount_low = 0;

                for i in 0..self.max_low_retry {
                    let stats = match self.get_stats().await {
                        Some(b) => b,
                        None => return SwitchType::Offline,
                    };

                    if stats.bitrate <= low.into() {
                        amount_low += 1;
                    }

                    debug!("LOW: {}/{}", amount_low, self.max_low_retry);

                    if i != self.max_low_retry - 1 {
                        time::sleep(Duration::from_secs(1)).await;
                    }
                }

                if amount_low == self.max_low_retry {
                    return SwitchType::Low;
                }

                return SwitchType::Normal;
            }
        }

        return SwitchType::Normal;
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for NodeMediaServer {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = match self.get_stats().await {
            Some(stats) => stats,
            None => return super::Bitrate { message: None },
        };

        if !stats.is_live {
            return super::Bitrate { message: None };
        }

        super::Bitrate {
            message: Some(format!("{}", stats.bitrate)),
        }
    }

    async fn source_info(&self) -> String {
        todo!()
    }
}

#[typetag::serde]
impl Bsl for NodeMediaServer {}
