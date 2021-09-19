use std::time::Duration;

use async_trait::async_trait;
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time;

use super::{Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
pub struct Stat {
    pub bitrate: i64,
    pub rtt: f64,
    pub dropped_pkts: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Belabox {
    /// URL to the BELABOX stats page (ex; http://127.0.0.1:8181/stats )
    pub stats_url: String,

    /// StreamID of the where you are publishing the feed. (ex; publish/live/feed1 )
    pub publisher: String,

    /// Max attempts to poll the bitrate every second on low bitrate.
    /// This will be used to make sure the stream is actually in a low
    /// bitrate state
    #[serde(default = "super::default_max_low_retry")]
    pub max_low_retry: u8,
}

impl Belabox {
    pub async fn get_stats(&self) -> Option<Stat> {
        let res = match reqwest::get(&self.stats_url).await {
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
        let data: Value = serde_json::from_str(&text).ok()?;
        let publisher = &data["publishers"][&self.publisher];

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
            return SwitchType::Offline;
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

        if let Some(rtt) = triggers.rtt {
            if stats.rtt >= rtt.into() {
                let mut amount_high = 0;

                for i in 0..self.max_low_retry {
                    let stats = match self.get_stats().await {
                        Some(b) => b,
                        None => return SwitchType::Offline,
                    };

                    if stats.bitrate >= rtt.into() {
                        amount_high += 1;
                    }

                    debug!("HIGH RTT: {}/{}", amount_high, self.max_low_retry);

                    if i != self.max_low_retry - 1 {
                        time::sleep(Duration::from_secs(1)).await;
                    }
                }

                if amount_high == self.max_low_retry {
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
impl StreamServersCommands for Belabox {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = match self.get_stats().await {
            Some(stats) => stats,
            None => return super::Bitrate { message: None },
        };

        let message = format!("{}, {} ms", stats.bitrate, stats.rtt.round());
        super::Bitrate {
            message: Some(message),
        }
    }

    async fn source_info(&self) -> String {
        todo!()
    }
}

#[typetag::serde]
impl Bsl for Belabox {}
