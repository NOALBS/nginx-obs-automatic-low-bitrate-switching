use async_trait::async_trait;
use log::error;
use serde::{Deserialize, Serialize};
use tracing::trace;

use super::{Bsl, StreamServersCommands, SwitchLogic, default_reqwest_client};
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
    stats: Option<FlowStats>,
}

#[derive(Deserialize, Debug)]
pub struct Peer {
    #[serde(default)]
    dead: usize,
    stats: PeerStats,
}

#[derive(Deserialize, Debug)]
pub struct FlowStats {
    bitrate: Option<usize>,
    bitrate_payload: Option<usize>,
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

    /// Use flow-level payload bitrate and traffic-weighted RTT for multipath sessions.
    #[serde(default)]
    pub multipath: bool,

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

    fn metrics(&self, flow: &Flowinstant) -> RistMetrics {
        if self.multipath {
            multipath_metrics(flow)
        } else {
            legacy_metrics(flow)
        }
    }
}

#[derive(Debug, PartialEq)]
struct RistMetrics {
    bitrate_kbps: u32,
    rtt_ms: f64,
}

fn legacy_metrics(flow: &Flowinstant) -> RistMetrics {
    let bitrate_bps = flow
        .peers
        .iter()
        .map(|peer| peer.stats.bitrate)
        .sum::<usize>();
    let rtt_ms = mean_rtt(flow.peers.iter().map(|peer| peer.stats.rtt));

    RistMetrics {
        bitrate_kbps: to_kbps(bitrate_bps),
        rtt_ms,
    }
}

fn multipath_metrics(flow: &Flowinstant) -> RistMetrics {
    let active_peers = flow
        .peers
        .iter()
        .filter(|peer| peer.dead == 0 && peer.stats.bitrate > 0)
        .collect::<Vec<_>>();

    let peer_bitrate_bps = active_peers
        .iter()
        .map(|peer| peer.stats.bitrate)
        .sum::<usize>();
    let bitrate_bps = flow
        .stats
        .as_ref()
        .and_then(|stats| stats.bitrate_payload.or(stats.bitrate))
        .unwrap_or(peer_bitrate_bps);

    let weighted_rtt = active_peers
        .iter()
        .filter(|peer| peer.stats.rtt.is_finite() && peer.stats.rtt >= 0.0)
        .fold((0.0, 0usize), |(weighted_sum, bitrate_sum), peer| {
            (
                weighted_sum + peer.stats.rtt * peer.stats.bitrate as f64,
                bitrate_sum.saturating_add(peer.stats.bitrate),
            )
        });
    let rtt_ms = if weighted_rtt.1 > 0 {
        weighted_rtt.0 / weighted_rtt.1 as f64
    } else {
        mean_rtt(
            flow.peers
                .iter()
                .filter(|peer| peer.dead == 0)
                .map(|peer| peer.stats.rtt),
        )
    };

    RistMetrics {
        bitrate_kbps: to_kbps(bitrate_bps),
        rtt_ms,
    }
}

fn mean_rtt(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values
        .filter(|rtt| rtt.is_finite() && *rtt >= 0.0)
        .fold((0.0, 0usize), |(sum, count), rtt| (sum + rtt, count + 1));

    if count == 0 { 0.0 } else { sum / count as f64 }
}

fn to_kbps(bitrate_bps: usize) -> u32 {
    u32::try_from(bitrate_bps / 1024).unwrap_or(u32::MAX)
}

fn classify(metrics: &RistMetrics, triggers: &Triggers) -> SwitchType {
    let bitrate = metrics.bitrate_kbps;
    let rtt = metrics.rtt_ms;

    if let Some(offline) = triggers.offline
        && bitrate > 0
        && bitrate <= offline
    {
        return SwitchType::Offline;
    }

    if let Some(rtt_offline) = triggers.rtt_offline
        && rtt >= rtt_offline.into()
    {
        return SwitchType::Offline;
    }

    if bitrate == 0 {
        return SwitchType::Offline;
    }

    if let Some(low) = triggers.low
        && bitrate <= low
    {
        return SwitchType::Low;
    }

    if let Some(rtt_trigger) = triggers.rtt
        && rtt >= rtt_trigger.into()
    {
        return SwitchType::Low;
    }

    SwitchType::Normal
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Rist {
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let flow = match self
            .get_stats()
            .await
            .and_then(|stats| stats.receiver_stats)
        {
            Some(stats) => stats.flowinstant,
            None => return SwitchType::Offline,
        };
        let metrics = self.metrics(&flow);
        classify(&metrics, triggers)
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Rist {
    async fn bitrate(&self) -> super::Bitrate {
        let flow = match self
            .get_stats()
            .await
            .and_then(|stats| stats.receiver_stats)
        {
            Some(stats) => stats.flowinstant,
            None => return super::Bitrate { message: None },
        };
        let metrics = self.metrics(&flow);
        let bitrate = metrics.bitrate_kbps;
        let rtt = metrics.rtt_ms;

        let message = format!("{}, {} ms", bitrate, rtt.round());
        super::Bitrate {
            message: Some(message),
        }
    }

    // TODO: Add more fields.
    async fn source_info(&self) -> Option<String> {
        let flow = self.get_stats().await?.receiver_stats?.flowinstant;
        let metrics = self.metrics(&flow);
        let bitrate = metrics.bitrate_kbps;
        let rtt = metrics.rtt_ms;

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

    fn flow_from_json(json: &str) -> Flowinstant {
        serde_json::from_str::<RistStats>(json)
            .unwrap()
            .receiver_stats
            .unwrap()
            .flowinstant
    }

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

    #[test]
    fn rist_config_defaults_to_legacy_metrics() {
        let rist: Rist =
            serde_json::from_str(r#"{"statsUrl":"http://localhost:8681/stats"}"#).unwrap();

        assert!(!rist.multipath);
    }

    #[test]
    fn legacy_metrics_keep_summing_peer_bitrate_and_averaging_rtt() {
        let flow = flow_from_json(MULTIPATH_STATS);

        let metrics = legacy_metrics(&flow);

        assert_eq!(metrics.bitrate_kbps, 7_109);
        assert!((metrics.rtt_ms - 73.171_614_689_821_08).abs() < 0.000_001);
    }

    #[test]
    fn multipath_metrics_use_payload_bitrate_and_traffic_weighted_rtt() {
        let flow = flow_from_json(MULTIPATH_STATS);

        let metrics = multipath_metrics(&flow);

        assert_eq!(metrics.bitrate_kbps, 5_870);
        assert!((metrics.rtt_ms - 70.704_672_409_555_5).abs() < 0.000_001);
    }

    #[test]
    fn low_traffic_high_rtt_peer_does_not_dominate_multipath_rtt() {
        let flow = flow_from_json(
            r#"{"receiver-stats":{"flowinstant":{"stats":{"bitrate_payload":7000000},"peers":[{"dead":0,"stats":{"rtt":50.0,"avg_rtt":50.0,"bitrate":6900000,"avg_bitrate":6900000}},{"dead":0,"stats":{"rtt":5000.0,"avg_rtt":5000.0,"bitrate":100000,"avg_bitrate":100000}}]}}}"#,
        );

        let metrics = multipath_metrics(&flow);

        assert_eq!(metrics.bitrate_kbps, 6_835);
        assert!((metrics.rtt_ms - 120.714_285_714_285_71).abs() < 0.000_001);
    }

    #[test]
    fn dead_peer_is_excluded_from_multipath_fallback_metrics() {
        let flow = flow_from_json(
            r#"{"receiver-stats":{"flowinstant":{"peers":[{"dead":0,"stats":{"rtt":60.0,"avg_rtt":60.0,"bitrate":4000000,"avg_bitrate":4000000}},{"dead":1,"stats":{"rtt":9000.0,"avg_rtt":9000.0,"bitrate":8000000,"avg_bitrate":8000000}}]}}}"#,
        );

        let metrics = multipath_metrics(&flow);

        assert_eq!(metrics.bitrate_kbps, 3_906);
        assert_eq!(metrics.rtt_ms, 60.0);
    }

    #[test]
    fn empty_peer_list_returns_offline_metrics_without_nan() {
        let flow = flow_from_json(
            r#"{"receiver-stats":{"flowinstant":{"stats":{"bitrate_payload":0},"peers":[]}}}"#,
        );

        let metrics = multipath_metrics(&flow);

        assert_eq!(metrics.bitrate_kbps, 0);
        assert_eq!(metrics.rtt_ms, 0.0);
        assert!(metrics.rtt_ms.is_finite());
    }

    #[test]
    fn healthy_multipath_flow_stays_normal_when_one_low_traffic_peer_has_high_rtt() {
        let flow = flow_from_json(
            r#"{"receiver-stats":{"flowinstant":{"stats":{"bitrate_payload":7000000},"peers":[{"dead":0,"stats":{"rtt":50.0,"avg_rtt":50.0,"bitrate":6900000,"avg_bitrate":6900000}},{"dead":0,"stats":{"rtt":5000.0,"avg_rtt":5000.0,"bitrate":100000,"avg_bitrate":100000}}]}}}"#,
        );
        let metrics = multipath_metrics(&flow);

        assert_eq!(classify(&metrics, &Triggers::default()), SwitchType::Normal);
    }

    #[test]
    fn actual_low_payload_switches_multipath_flow_to_low() {
        let metrics = RistMetrics {
            bitrate_kbps: 800,
            rtt_ms: 50.0,
        };

        assert_eq!(classify(&metrics, &Triggers::default()), SwitchType::Low);
    }

    #[test]
    fn zero_payload_switches_multipath_flow_offline() {
        let metrics = RistMetrics {
            bitrate_kbps: 0,
            rtt_ms: 0.0,
        };

        assert_eq!(
            classify(&metrics, &Triggers::default()),
            SwitchType::Offline
        );
    }

    const MULTIPATH_STATS: &str = r#"{
        "receiver-stats": {
            "flowinstant": {
                "peers": [
                    {
                        "dead": 0,
                        "stats": {
                            "rtt": 54.974101780060245,
                            "avg_rtt": 47.3954839757791,
                            "bitrate": 4133725,
                            "avg_bitrate": 3481568
                        }
                    },
                    {
                        "dead": 0,
                        "stats": {
                            "rtt": 91.3691273995819,
                            "avg_rtt": 88.26277093630755,
                            "bitrate": 3146749,
                            "avg_bitrate": 2595381
                        }
                    }
                ],
                "stats": {
                    "bitrate": 6011059,
                    "bitrate_payload": 6011059
                }
            }
        },
        "schema_version": 5
    }"#;
}
