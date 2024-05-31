use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{error, trace};

use super::{default_reqwest_client, Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StreamStats {
    pub name: String,
    pub source: Source,
    pub bytes_received: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SrtStats {
    pub id: String,
    pub created: String,
    pub remote_addr: String,
    pub state: String,
    pub path: String,
    pub query: String,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_sent_unique: u64,
    pub packets_received_unique: u64,
    pub packets_send_loss: u64,
    pub packets_received_loss: u64,
    pub packets_retrans: u64,
    pub packets_received_retrans: u64,
    #[serde(rename = "packetsSentACK")]
    pub packets_sent_ack: u64,
    #[serde(rename = "packetsReceivedACK")]
    pub packets_received_ack: u64,
    #[serde(rename = "packetsSentNAK")]
    pub packets_sent_nak: u64,
    #[serde(rename = "packetsReceivedNAK")]
    pub packets_received_nak: u64,
    #[serde(rename = "packetsSentKM")]
    pub packets_sent_km: u64,
    #[serde(rename = "packetsReceivedKM")]
    pub packets_received_km: u64,
    pub us_snd_duration: u64,
    pub packets_send_drop: u64,
    pub packets_received_drop: u64,
    pub packets_received_undecrypt: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub bytes_sent_unique: u64,
    pub bytes_received_unique: u64,
    pub bytes_received_loss: u64,
    pub bytes_retrans: u64,
    pub bytes_received_retrans: u64,
    pub bytes_send_drop: u64,
    pub bytes_received_drop: u64,
    pub bytes_received_undecrypt: u64,
    pub us_packets_send_period: f64,
    pub packets_flow_window: u64,
    pub packets_flight_size: u64,
    #[serde(rename = "msRTT")]
    pub ms_rtt: f64,
    pub mbps_send_rate: f64,
    pub mbps_receive_rate: f64,
    pub mbps_link_capacity: f64,
    pub bytes_avail_send_buf: u64,
    pub bytes_avail_receive_buf: u64,
    #[serde(rename = "mbpsMaxBW")]
    pub mbps_max_bw: f64,
    #[serde(rename = "byteMSS")]
    pub byte_mss: u64,
    pub packets_send_buf: u64,
    pub bytes_send_buf: u64,
    pub ms_send_buf: u64,
    pub ms_send_tsb_pd_delay: u64,
    pub packets_receive_buf: u64,
    pub bytes_receive_buf: u64,
    pub ms_receive_buf: u64,
    pub ms_receive_tsb_pd_delay: u64,
    pub packets_reorder_tolerance: u64,
    pub packets_received_avg_belated_time: u64,
    pub packets_send_loss_rate: f64,
    pub packets_received_loss_rate: f64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub bitrate: u32,
    pub srt: Option<SrtStats>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mediamtx {
    /// URL to MediaMTX stats page (ex; http://localhost:9997/v3/paths/get/mystream )
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
    pub async fn get_stats(&self) -> Option<Stats> {
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

        let stream = match res.json::<StreamStats>().await {
            Ok(stats) => stats,
            Err(e) => {
                error!("Error parsing stats ({}) {}", self.stats_url, e);
                return None;
            }
        };

        let mut stats = Stats::default();

        if stream.source.kind == "srtConn" {
            stats.srt = self.get_srt_stats(&stream.source.id).await;
        }

        let mut cache = self.cache.lock().unwrap();

        if stream.bytes_received == cache.prev_bytes_received {
            return None;
        }

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
        stats.bitrate = cache.bitrate;
        Some(stats)
    }

    pub async fn get_srt_stats(&self, id: &str) -> Option<SrtStats> {
        let stats_url: Vec<&str> = self.stats_url.split("/v3").collect();
        let stats_url = format!("{}/v3/srtconns/get/{id}", stats_url.first()?);

        let res = match self.client.get(stats_url.clone()).send().await {
            Ok(res) => res,
            Err(_) => {
                error!("Stats page ({}) is unreachable", stats_url);
                return None;
            }
        };

        if res.status() == reqwest::StatusCode::INTERNAL_SERVER_ERROR {
            return None;
        }

        if res.status() != reqwest::StatusCode::OK {
            error!("Error accessing SRT stats page ({})", stats_url);
            return None;
        }

        let stats = res.json::<SrtStats>().await.ok()?;

        trace!("{:#?}", stats);
        Some(stats)
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Mediamtx {
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let Some(stats) = self.get_stats().await else {
            return SwitchType::Offline;
        };

        let ms_rtt = stats.srt.map(|s| s.ms_rtt);

        if let Some(offline) = triggers.offline {
            if stats.bitrate > 0 && stats.bitrate <= offline {
                return SwitchType::Offline;
            }
        }

        if let Some(rtt_offline) = triggers.rtt_offline {
            if let Some(ms_rtt) = ms_rtt {
                if ms_rtt >= rtt_offline.into() {
                    return SwitchType::Offline;
                }
            }
        }

        if stats.bitrate == 0 {
            return SwitchType::Previous;
        }

        if let Some(low) = triggers.low {
            if stats.bitrate <= low {
                return SwitchType::Low;
            }
        }

        if let Some(rtt) = triggers.rtt {
            if let Some(ms_rtt) = ms_rtt {
                if ms_rtt >= rtt.into() {
                    return SwitchType::Low;
                }
            }
        }

        SwitchType::Normal
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Mediamtx {
    async fn bitrate(&self) -> super::Bitrate {
        let Some(stats) = self.get_stats().await else {
            return super::Bitrate { message: None };
        };

        let mut message = format!("{}", stats.bitrate);

        if let Some(srt) = stats.srt {
            message += &format!(", {} ms", srt.ms_rtt.round());
        }

        super::Bitrate {
            message: Some(message),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let stats = self.get_stats().await?;
        let bitrate = stats.bitrate;

        let mut info = format!("{} Kbps", bitrate);

        if let Some(srt) = stats.srt {
            let rtt = format!(", {} ms", srt.ms_rtt.round());

            let mbps = format!("Receiving rate {:.2} Mbps", srt.mbps_receive_rate);

            let pkt = format!(
                "dropped {}, loss {}, retrans {}",
                srt.packets_received_drop, srt.packets_received_loss, srt.packets_received_retrans
            );

            let latency = format!(
                "Latency send {} ms receive {} ms",
                srt.ms_send_tsb_pd_delay, srt.ms_receive_tsb_pd_delay
            );

            info += &format!("{} | {} | {} | {}", rtt, mbps, pkt, latency);
        }

        Some(info)
    }
}

#[typetag::serde]
impl Bsl for Mediamtx {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
