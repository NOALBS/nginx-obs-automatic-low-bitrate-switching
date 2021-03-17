use super::{StreamServersCommands, SwitchLogic, SwitchType, Triggers, BSL};
use async_trait::async_trait;
use log::{error, trace};
use serde::Deserialize;
use serde_json::*;

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

        let stream: Stat = serde_json::from_value(publisher.to_owned()).ok()?;
        // let stream: Stat = match serde_json::from_value(publisher.to_owned()) {
        //     Ok(stats) => stats,
        //     Err(error) => {
        //         trace!("{}", &data);
        //         error!("Error parsing stats {}", error);
        //         return None;
        //     }
        // };

        trace!("{:#?}", stream);
        Some(stream)
    }
}

#[async_trait]
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
impl StreamServersCommands for SrtLiveServer {
    async fn bitrate(&self) -> String {
        let stats = if let Some(stats) = self.get_stats().await {
            stats
        } else {
            return "Offline".to_string();
        };

        format!("bitrate {} Kbps, RTT {} ms", stats.bitrate, stats.rtt)
    }

    async fn source_info(&self) -> String {
        todo!()
    }
}

impl BSL for SrtLiveServer {}
