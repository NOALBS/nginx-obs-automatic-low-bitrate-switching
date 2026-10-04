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
    /// Not reported for bonded connections, see `peers` instead
    #[serde(default)]
    pub rtt: Option<f64>,
    pub uptime: u64,
    /// Individual links of a bonded connection
    #[serde(default)]
    pub peers: Vec<Peer>,
}

#[derive(Deserialize, Debug)]
pub struct Peer {
    pub bitrate: u32,
    pub rtt: f64,
}

impl Stat {
    /// RTT of the connection, for bonded connections the bitrate-weighted
    /// average of all active peers with a valid RTT
    pub fn rtt(&self) -> Option<f64> {
        if let Some(rtt) = self.rtt {
            return Some(rtt);
        }

        let (weighted_sum, bitrate_sum) = self
            .peers
            .iter()
            .filter(|peer| peer.bitrate > 0 && peer.rtt.is_finite() && peer.rtt >= 0.0)
            .fold((0.0, 0u64), |(weighted_sum, bitrate_sum), peer| {
                (
                    weighted_sum + peer.rtt * peer.bitrate as f64,
                    bitrate_sum + peer.bitrate as u64,
                )
            });

        if bitrate_sum == 0 {
            return None;
        }

        Some(weighted_sum / bitrate_sum as f64)
    }

    fn rtt_message(&self) -> String {
        match self.rtt() {
            Some(rtt) => format!(", {} ms", rtt.round()),
            None => String::new(),
        }
    }
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

        if let Some(offline) = triggers.offline
            && stats.bitrate > 0
            && stats.bitrate <= offline
        {
            return SwitchType::Offline;
        }

        let rtt = stats.rtt();

        if let Some(rtt_offline) = triggers.rtt_offline
            && let Some(rtt) = rtt
            && rtt >= rtt_offline.into()
        {
            return SwitchType::Offline;
        }

        if stats.bitrate == 0 {
            return SwitchType::Offline;
        }

        if stats.bitrate == 1 {
            return SwitchType::Previous;
        }

        if let Some(low) = triggers.low
            && stats.bitrate <= low
        {
            return SwitchType::Low;
        }

        if let Some(rtt_trigger) = triggers.rtt
            && let Some(rtt) = rtt
            && rtt >= rtt_trigger.into()
        {
            return SwitchType::Low;
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

        let message = format!("{} Kbps{}", stats.bitrate, stats.rtt_message());
        super::Bitrate {
            message: Some(message),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let stats = self.get_stats().await?;

        let bitrate = format!(
            "{} Kbps{} at {} ms latency",
            stats.bitrate,
            stats.rtt_message(),
            stats.latency
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_connection_rtt() {
        let stat: Stat = serde_json::from_str(
            r#"{"bitrate":3000,"buffer":335,"dropped_pkts":0,"latency":3000,"rtt":42.4,"uptime":1}"#,
        )
        .unwrap();

        assert_eq!(stat.rtt(), Some(42.4));
    }

    #[test]
    fn weights_bonded_peer_rtt_by_bitrate() {
        let stat: Stat = serde_json::from_str(
            r#"{"bitrate":3,"buffer":335,"dropped_pkts":0,"latency":3000,"peers":[{"bitrate":479,"connection_id":"4d2fc15b","jitter":12.069,"rtt":20.501,"throughput":483,"uptime":1},{"bitrate":284,"connection_id":"bf51f69f","jitter":12.165,"rtt":84.249,"throughput":288,"uptime":0}],"quality":100.0,"throughput":771,"uptime":1}"#,
        )
        .unwrap();

        let expected = (20.501 * 479.0 + 84.249 * 284.0) / 763.0;
        assert!((stat.rtt().unwrap() - expected).abs() < 0.000_001);
    }

    #[test]
    fn ignores_inactive_peers() {
        let stat: Stat = serde_json::from_str(
            r#"{"bitrate":500,"buffer":335,"dropped_pkts":0,"latency":3000,"peers":[{"bitrate":500,"rtt":60.0},{"bitrate":0,"rtt":5.0}],"uptime":1}"#,
        )
        .unwrap();

        assert_eq!(stat.rtt(), Some(60.0));
    }

    #[test]
    fn no_active_peers_is_none() {
        let stat: Stat = serde_json::from_str(
            r#"{"bitrate":0,"buffer":335,"dropped_pkts":0,"latency":3000,"peers":[{"bitrate":0,"rtt":20.0}],"uptime":1}"#,
        )
        .unwrap();

        assert_eq!(stat.rtt(), None);
    }

    #[test]
    fn missing_rtt_without_peers_is_none() {
        let stat: Stat = serde_json::from_str(
            r#"{"bitrate":3000,"buffer":335,"dropped_pkts":0,"latency":3000,"uptime":1}"#,
        )
        .unwrap();

        assert_eq!(stat.rtt(), None);
    }
}
