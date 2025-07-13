use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, trace};

use super::{Bsl, StreamServersCommands, SwitchLogic, default_reqwest_client};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
pub struct Stat {
    pub bitrate: u32,
    pub buffer: u32,
    pub dropped_pkts: u64,
    pub latency: u16,
    pub rtt: f64,
    pub uptime: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenIRL {
    /// URL to the OpenIRL stats page (ex; http://127.0.0.1:8080/stats/play_38f01143fc4049c5836d7f7dcaf1a31f )
    pub stats_url: String,

    /// Client to make HTTP requests with
    #[serde(skip, default = "default_reqwest_client")]
    pub client: reqwest::Client,
}

impl OpenIRL {
    pub async fn get_stats(&self) -> Option<Stat> {
        let res = match self.client.get(&self.stats_url).send().await {
            Ok(res) => res,
            Err(e) => {
                error!("Stats page is unreachable, {}", e);
                return None;
            }
        };

        if res.status() != reqwest::StatusCode::OK {
            error!("Error accessing stats page ({})", self.stats_url);
            return None;
        }

        let text = res.text().await.ok()?;
        let data: Value = serde_json::from_str(&text).ok()?;
        
        // Check if "publisher" field exists - if not, stream is offline
        let publisher = match data.get("publisher") {
            Some(publisher) => publisher,
            None => {
                // Publisher is offline - return None (standard pattern)
                return None;
            }
        };

        let stream: Stat = match serde_json::from_value(publisher.to_owned()) {
            Ok(stats) => stats,
            Err(error) => {
                trace!("{}", &data);
                error!("Error parsing stats ({}) {}", self.stats_url, error);
                return None;
            }
        };

        trace!("{:#?}", stream);
        Some(stream)
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for OpenIRL {
    /// Which scene to switch to
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let stats = match self.get_stats().await {
            Some(b) => b,
            None => return SwitchType::Offline,
        };

        if let Some(offline) = triggers.offline {
            if stats.bitrate > 0 && stats.bitrate <= offline.into() {
                return SwitchType::Offline;
            }
        }

        if let Some(rtt_offline) = triggers.rtt_offline {
            if stats.rtt >= rtt_offline.into() {
                return SwitchType::Offline;
            }
        }

        if stats.bitrate == 0 {
            return SwitchType::Offline;
        }

        if stats.bitrate == 1 {
            return SwitchType::Previous;
        }

        if let Some(low) = triggers.low {
            if stats.bitrate <= low.into() {
                return SwitchType::Low;
            }
        }

        if let Some(rtt) = triggers.rtt {
            if stats.rtt >= rtt.into() {
                return SwitchType::Low;
            }
        }

        return SwitchType::Normal;
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for OpenIRL {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = match self.get_stats().await {
            Some(stats) => stats,
            None => return super::Bitrate { message: None },
        };

        let message = format!("{} Kbps, {} ms", stats.bitrate, stats.rtt.round());
        super::Bitrate {
            message: Some(message),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let stats = self.get_stats().await?;

        let bitrate = format!("{} Kbps, {} ms at {} ms latency", stats.bitrate, stats.rtt.round(), stats.latency);
        let dropped = format!("dropped {} packets", stats.dropped_pkts);

        Some(format!("{} | {}", bitrate, dropped))
    }
}

#[typetag::serde]
impl Bsl for OpenIRL {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
