use async_trait::async_trait;
use log::{error, trace};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Stat {
    pub bitrate: i64,
    pub bytes_rcv_drop: u64,
    pub bytes_rcv_loss: u64,
    pub mbps_bandwidth: f64,
    pub mbps_recv_rate: f64,
    pub ms_rcv_buf: i32,
    pub pkt_rcv_drop: i32,
    pub pkt_rcv_loss: i32,
    pub rtt: f64,
    pub uptime: i64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SrtLiveServer {
    /// URL to SLS stats page (ex; http://127.0.0.1:8181/stats )
    pub stats_url: String,

    /// StreamID of the where you are publishing the feed. (ex; publish/live/feed1 )
    pub publisher: String,
}

impl SrtLiveServer {
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

        let stream: Stat = serde_json::from_value(publisher.to_owned()).ok()?;
        // let stream: Stat = match serde_json::from_value(publisher.to_owned()) {
        //     Ok(stats) => stats,
        //     Err(error) => {
        //         trace!("{}", &data);
        //         error!("Error parsing stats ({}) {}", self.stats_url, error);
        //         return None;
        //     }
        // };

        trace!("{:#?}", stream);
        Some(stream)
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for SrtLiveServer {
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
impl StreamServersCommands for SrtLiveServer {
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

    async fn source_info(&self) -> Option<String> {
        let stats = self.get_stats().await?;

        let bitrate = format!("{} Kbps, {} ms", stats.bitrate, stats.rtt.round());

        let mbps = format!(
            "Estimated bandwidth {} Mbps, Receiving rate {:.2} Mbps",
            stats.mbps_bandwidth.round(),
            stats.mbps_recv_rate
        );

        let pkt = format!(
            "{} dropped, {} loss",
            stats.pkt_rcv_drop, stats.pkt_rcv_loss
        );

        // The ms of acknowledged packets in the receiver's buffer
        let ms_buf = format!("{} ms buffer", stats.ms_rcv_buf);

        Some(format!("{} | {} | {} |  {}", bitrate, mbps, pkt, ms_buf))
    }
}

#[typetag::serde]
impl Bsl for SrtLiveServer {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
