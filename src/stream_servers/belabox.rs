use crate::db;

use super::{Bsl, StreamServersCommands, SwitchLogic, SwitchType, Triggers};
use async_trait::async_trait;
use log::{error, trace};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Stat {
    pub bitrate: i64,
    pub rtt: f64,
    pub dropped_pkts: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Belabox {
    /// URL to the BELABOX stats page (ex; http://127.0.0.1:8181/stats )
    pub stats_url: String,

    /// StreamID of the where you are publishing the feed. (ex; publish/live/feed1 )
    pub publisher: String,

    /// A name to differentiate in case of multiple stream servers
    pub name: String,

    /// Priority
    pub priority: i32,
}

impl Belabox {
    pub async fn get_stats(&self) -> Option<Stat> {
        let res = match reqwest::get(&self.stats_url).await {
            Ok(res) => res,
            Err(_) => {
                error!("Stats page is unreachable");
                return None;
            }
        };

        if res.status() != reqwest::StatusCode::OK {
            error!("Error accessing stats page");
            return None;
        }

        let text = res.text().await.ok()?;
        let data: Value = serde_json::from_str(&text).ok()?;
        let publisher = &data["publishers"][&self.publisher];
        let stream: Stat = match serde_json::from_value(publisher.to_owned()) {
            Ok(stats) => stats,
            Err(error) => {
                trace!("{}", &data);
                error!("Error parsing stats {}", error);
                return None;
            }
        };

        trace!("{:#?}", stream);
        Some(stream)
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Belabox {
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

        if stats.bitrate == 0 {
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

    fn priority(&self) -> i32 {
        self.priority
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Belabox {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = if let Some(stats) = self.get_stats().await {
            stats
        } else {
            return super::Bitrate {
                name: &self.name,
                message: None,
                priority: self.priority,
            };
        };

        let message = format!("{}, RTT {} ms", stats.bitrate, stats.rtt.round());
        super::Bitrate {
            name: &self.name,
            message: Some(message),
            priority: self.priority,
        }
    }

    async fn source_info(&self) -> String {
        todo!()
    }
}

#[typetag::serde]
impl Bsl for Belabox {}

impl From<db::StreamServer> for Belabox {
    fn from(item: db::StreamServer) -> Self {
        Self {
            stats_url: item.stats_url,
            publisher: item.application,
            name: item.name,
            priority: item.priority,
        }
    }
}
