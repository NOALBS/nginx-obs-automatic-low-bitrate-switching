use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::error;

use super::{default_reqwest_client, Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct NimbleSrtStats {
    pub srt_receivers: Vec<SrtReceiver>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SrtReceiver {
    pub id: String,
    pub state: String,
    pub stats: Stats,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub time: u64,
    pub window: Window,
    pub link: Link,
    pub recv: Recv,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub flow: u64,
    pub congestion: u64,
    pub flight: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub rtt: f64,
    pub mbps_bandwidth: f64,
    pub mbps_max_bandwidth: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Recv {
    pub packets_received: u64,
    pub packets_lost: u64,
    pub packets_dropped: u64,
    pub packets_belated: u64,
    #[serde(rename(deserialize = "NAKsSent"))]
    pub naks_sent: u64,
    pub bytes_received: u64,
    pub bytes_lost: u64,
    pub bytes_dropped: u64,
    pub mbps_rate: f64,
}

#[derive(Deserialize, Debug)]
pub struct NimbleRtmpStats {
    pub app: String,
    pub streams: Vec<Streams>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Streams {
    pub acodec: Option<String>,
    pub vcodec: String,
    pub publish_time: String,
    pub bandwidth: String,
    pub protocol: String,
    pub resolution: String,
    pub strm: String,
}

pub struct Stat {
    pub srt: SrtReceiver,
    pub rtmp: Streams,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Nimble {
    /// UDP listener ID (Usually IP:Port)
    pub id: String,

    /// URL to nimble API
    pub stats_url: String,

    /// Outgoing stream "Application Name"
    pub application: String,

    /// Outgoing stream "Stream Name"
    pub key: String,

    /// Client to make HTTP requests with
    #[serde(skip, default = "default_reqwest_client")]
    pub client: reqwest::Client,
}

impl Nimble {
    pub async fn get_stats(&self) -> Option<Stat> {
        let url = format!("{}/manage/srt_receiver_stats", &self.stats_url);

        let res = match self.client.get(&url).send().await {
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
        let srt_stats: NimbleSrtStats = serde_json::from_str(&text).ok()?;

        let srt_receiver = srt_stats
            .srt_receivers
            .iter()
            .find(|x| x.id.contains(&self.id))?;

        if srt_receiver.state == "disconnected" {
            return None;
        }

        // RTMP status for bitrate. srt_receiver_stats seems to give an averaged number that isn't as useful.
        // Probably requires nimble to be configured to make the video from SRT available on RTMP even though it's not used anywhere
        let url = format!("{}/manage/rtmp_status", &self.stats_url);

        let res = match reqwest::get(&url).await {
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
        let rtmp_stats: Vec<NimbleRtmpStats> = serde_json::from_str(&text).ok()?;

        let rtmp_app = rtmp_stats.iter().find(|x| x.app == self.application)?;
        let rtmp_stream = rtmp_app.streams.iter().find(|x| x.strm == self.key)?;

        let stat = Stat {
            srt: srt_receiver.to_owned(),
            rtmp: rtmp_stream.to_owned(),
        };

        Some(stat)
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Nimble {
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let stats = match self.get_stats().await {
            Some(b) => b,
            None => return SwitchType::Offline,
        };

        let bitrate = stats.rtmp.bandwidth.parse::<u32>().unwrap();
        let bitrate = bitrate / 1024;

        if let Some(offline) = triggers.offline {
            if bitrate > 0 && bitrate <= offline {
                return SwitchType::Offline;
            }
        }

        if let Some(rtt_offline) = triggers.rtt_offline {
            if stats.srt.stats.link.rtt >= rtt_offline.into() {
                return SwitchType::Offline;
            }
        }

        if bitrate == 0 {
            return SwitchType::Normal;
        }

        if let Some(low) = triggers.low {
            if bitrate <= low {
                return SwitchType::Low;
            }
        }

        if let Some(rtt) = triggers.rtt {
            if stats.srt.stats.link.rtt >= rtt.into() {
                return SwitchType::Low;
            }
        }

        return SwitchType::Normal;
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Nimble {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = match self.get_stats().await {
            Some(stats) => stats,
            None => return super::Bitrate { message: None },
        };

        let bitrate = stats.rtmp.bandwidth.parse::<u32>().unwrap();
        let bitrate = bitrate / 1024;

        let message = format!("{}, {} ms", bitrate, stats.srt.stats.link.rtt.round());
        super::Bitrate {
            message: Some(message),
        }
    }

    async fn source_info(&self) -> Option<String> {
        self.bitrate().await.message
    }
}

#[typetag::serde]
impl Bsl for Nimble {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
