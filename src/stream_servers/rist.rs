use async_trait::async_trait;
use log::error;
use serde::{Deserialize, Serialize};
use tracing::trace;

use super::{default_reqwest_client, Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
pub struct RistStats {
    #[serde(rename = "receiver-stats")]
    receiver_stats: Option<ReceiverStats>,
}

#[derive(Deserialize, Debug)]
pub struct ReceiverStats {
    flowinstant: Flowinstant,
}

#[derive(Deserialize, Debug)]
pub struct Flowinstant {
    peers: Vec<Peer>,
}

#[derive(Deserialize, Debug)]
pub struct Peer {
    stats: PeerStats,
}

#[derive(Deserialize, Debug)]
pub struct PeerStats {
    pub rtt: f64,
    pub avg_rtt: f64,
    pub bitrate: usize,
    pub avg_bitrate: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Rist {
    /// URL to RIST stats page
    pub stats_url: String,

    /// Client to make HTTP requests with
    #[serde(skip, default = "default_reqwest_client")]
    pub client: reqwest::Client,
}

impl Rist {
    pub async fn get_stats(&self) -> Option<RistStats> {
        let res = match self.client.get(&self.stats_url).send().await {
            Ok(res) => res,
            Err(e) => {
                error!("Stats page ({}) is unreachable ({})", self.stats_url, e);
                return None;
            }
        };

        if res.status() != reqwest::StatusCode::OK {
            error!("Error accessing stats page ({})", self.stats_url);
            return None;
        }

        let stream = match res.json::<RistStats>().await {
            Ok(stats) => stats,
            Err(e) => {
                error!("Error parsing stats ({}) {}", self.stats_url, e);
                return None;
            }
        };

        trace!("{:#?}", stream);
        Some(stream)
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Rist {
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let stats = match self
            .get_stats()
            .await
            .and_then(|stats| stats.receiver_stats)
        {
            Some(s) => s.flowinstant.peers,
            None => return SwitchType::Offline,
        };

        let bitrate: u32 = (stats.iter().map(|p| p.stats.bitrate).sum::<usize>() / 1024)
            .try_into()
            .unwrap();
        let rtt = stats.iter().map(|p| p.stats.rtt).sum::<f64>() / stats.len() as f64;

        if let Some(offline) = triggers.offline {
            if bitrate > 0 && bitrate <= offline {
                return SwitchType::Offline;
            }
        }

        if let Some(rtt_offline) = triggers.rtt_offline {
            if rtt >= rtt_offline.into() {
                return SwitchType::Offline;
            }
        }

        if bitrate == 0 {
            return SwitchType::Offline;
        }

        if let Some(low) = triggers.low {
            if bitrate <= low {
                return SwitchType::Low;
            }
        }

        if let Some(rtt_trigger) = triggers.rtt {
            if rtt >= rtt_trigger.into() {
                return SwitchType::Low;
            }
        }

        SwitchType::Normal
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Rist {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = match self
            .get_stats()
            .await
            .and_then(|stats| stats.receiver_stats)
        {
            Some(s) => s.flowinstant.peers,
            None => return super::Bitrate { message: None },
        };

        let bitrate: u32 = (stats.iter().map(|p| p.stats.bitrate).sum::<usize>() / 1024)
            .try_into()
            .unwrap();
        let rtt = stats.iter().map(|p| p.stats.rtt).sum::<f64>() / stats.len() as f64;

        let message = format!("{}, {} ms", bitrate, rtt.round());
        super::Bitrate {
            message: Some(message),
        }
    }

    // TODO: Add more fields.
    async fn source_info(&self) -> Option<String> {
        let stats = self.get_stats().await?.receiver_stats?.flowinstant.peers;

        let bitrate: u32 = (stats.iter().map(|p| p.stats.bitrate).sum::<usize>() / 1024)
            .try_into()
            .unwrap();
        let rtt = stats.iter().map(|p| p.stats.rtt).sum::<f64>() / stats.len() as f64;

        let bitrate = format!("{} Kbps, {} ms", bitrate, rtt.round());

        Some(bitrate)
    }
}

#[typetag::serde]
impl Bsl for Rist {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_stream() {
        let s = r#"{"receiver-stats":null}"#;
        let parsed: RistStats = serde_json::from_str(s).unwrap();
        println!("{:#?}", parsed);

        assert!(parsed.receiver_stats.is_none());
    }

    #[test]
    fn stream() {
        let s = r#"{"receiver-stats":{"flowinstant":{"flow_id":678204162,"dead":0,"stats":{"quality":100,"received":670,"dropped_late":0,"dropped_full":0,"missing":0,"recovered_total":0,"reordered":0,"retries":0,"recovered_one_nack":0,"recovered_two_nacks":0,"recovered_three_nacks":0,"recovered_four_nacks":0,"recovered_more_nacks":0,"lost":0,"avg_buffer_time":984,"duplicates":0,"missing_queue":0,"missing_queue_max":3571,"min_inter_packet_spacing":8,"cur_inter_packet_spacing":1574,"max_inter_packet_spacing":97897,"bitrate":6553449},"peers":[{"id":12,"dead":0,"stats":{"received_data":670,"received_rtcp":19,"sent_rtcp":20,"rtt":282.53341341155823,"avg_rtt":293.7726524441282,"bitrate":6651751,"avg_bitrate":6366555}}]}}}"#;
        let parsed: RistStats = serde_json::from_str(s).unwrap();

        assert!(
            parsed.receiver_stats.is_some(),
            "Receiver stats should be present"
        );

        let receiver_stats = parsed.receiver_stats.as_ref().unwrap();
        assert_eq!(
            receiver_stats.flowinstant.peers.len(),
            1,
            "There should be one peer"
        );

        let peer_stats = &receiver_stats.flowinstant.peers[0].stats;
        assert_eq!(peer_stats.bitrate, 6651751, "Bitrate should be 6651751");
    }
}
