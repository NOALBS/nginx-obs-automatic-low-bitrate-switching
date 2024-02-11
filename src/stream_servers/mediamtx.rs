use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{error, trace};

use super::{default_reqwest_client, Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub name: String,
    pub bytes_received: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mediamtx {
    /// URL to SLS stats page (ex; http://127.0.0.1:8181/stats )
    pub stats_url: String,

    /// Client to make HTTP requests with
    #[serde(skip, default = "default_reqwest_client")]
    pub client: reqwest::Client,

    #[serde(skip)]
    pub cache: Arc<Mutex<Cache>>,
}

pub struct Cache {
    // The last total bytes received
    pub prev_bytes_received: u64,

    // The last time we checked the stats
    pub timestamp: std::time::Instant,

    // The current bitrate
    pub bitrate: u32,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            prev_bytes_received: 0,
            timestamp: std::time::Instant::now(),
            bitrate: 0,
        }
    }
}

impl Mediamtx {
    pub async fn get_stats(&self) -> Option<u32> {
        let res = match self.client.get(&self.stats_url).send().await {
            Ok(res) => res,
            Err(_) => {
                error!("Stats page ({}) is unreachable", self.stats_url);
                return None;
            }
        };

        if res.status() == reqwest::StatusCode::INTERNAL_SERVER_ERROR {
            return None;
        }

        if res.status() != reqwest::StatusCode::OK {
            error!("Error accessing stats page ({})", self.stats_url);
            return None;
        }

        let stream = match res.json::<Stats>().await {
            Ok(stats) => stats,
            Err(e) => {
                error!("Error parsing stats ({}) {}", self.stats_url, e);
                return None;
            }
        };

        let mut cache = self.cache.lock().unwrap();
        let elapsed = cache.timestamp.elapsed();
        if elapsed >= std::time::Duration::from_secs(1) {
            if stream.bytes_received > cache.prev_bytes_received {
                let diff_bits = (stream.bytes_received - cache.prev_bytes_received) * 8;
                let bits_per_second = diff_bits as f64 / elapsed.as_secs_f64();
                let kbps = bits_per_second / 1024.0;
                cache.bitrate = kbps as u32;
            }

            cache.prev_bytes_received = stream.bytes_received;
            cache.timestamp = std::time::Instant::now();
        }

        trace!("{:#?}", stream);
        Some(cache.bitrate)
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Mediamtx {
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let bitrate = match self.get_stats().await {
            Some(b) => b,
            None => return SwitchType::Offline,
        };

        if let Some(offline) = triggers.offline {
            if bitrate > 0 && bitrate <= offline {
                return SwitchType::Offline;
            }
        }

        if bitrate == 0 {
            return SwitchType::Previous;
        }

        if let Some(low) = triggers.low {
            if bitrate <= low {
                return SwitchType::Low;
            }
        }

        SwitchType::Normal
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Mediamtx {
    async fn bitrate(&self) -> super::Bitrate {
        let bitrate = match self.get_stats().await {
            Some(stats) => stats,
            None => return super::Bitrate { message: None },
        };

        super::Bitrate {
            message: Some(format!("{}", bitrate)),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let bitrate = self.get_stats().await?;
        Some(format!("{} Kbps", bitrate))
    }
}

#[typetag::serde]
impl Bsl for Mediamtx {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
