use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{error, trace};

use super::{Bsl, StreamServersCommands, SwitchLogic, default_reqwest_client};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
#[serde(tag = "service")]
pub enum Stat {
    #[serde(rename = "SRT")]
    Srt {
        status: String,
        timestamp: String,
        publishers: Vec<SrtPublisher>,
        overall: SrtOverall,
    },
    #[serde(rename = "RTMP")]
    Rtmp {
        server: RtmpServer,
        applications: Vec<RtmpApplication>,
        timestamp: String,
    },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SrtPublisher {
    stream: String,
    bitrate: u64,
    #[serde(rename = "bitrate_mbps")]
    bitrate_mbps: String,
    bytes_received: u64,
    bytes_dropped: u64,
    bytes_lost: u64,
    mbps_bandwidth: f64,
    mbps_recv_rate: f64,
    rtt: f64,
    uptime: String,
    packet_stats: PacketStats,
    buffer: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PacketStats {
    packets_dropped: u64,
    packets_lost: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SrtOverall {
    total_publishers: u64,
    total_bytes_received: u64,
    total_bytes_dropped: u64,
    total_bytes_lost: u64,
    average_bitrate: String,
    total_uptime: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtmpServer {
    nginx_version: String,
    rtmp_version: String,
    compiler: String,
    built: String,
    pid: String,
    uptime: String,
    connections: Connections,
    bandwidth: Bandwidth,
    bytes: Bytes,
}

#[derive(Deserialize, Debug)]
pub struct Connections {
    accepted: u64,
    current: u64,
}

#[derive(Deserialize, Debug)]
pub struct Bandwidth {
    incoming: String,
    outgoing: String,
}

#[derive(Deserialize, Debug)]
pub struct Bytes {
    incoming: u32,
    outgoing: u32,
}

#[derive(Deserialize, Debug)]
pub struct RtmpApplication {
    name: String,
    live: RtmpLive,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtmpLive {
    streams: Vec<RtmpStream>,
    total_clients: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtmpStream {
    name: String,
    duration: String,
    bandwidth: Bandwidth,
    bytes: Bytes,
    audio: RtmpAudio,
    video: RtmpVideo,
    clients: Vec<RtmpClient>,
    status: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtmpAudio {
    codec: String,
    profile: String,
    channels: u64,
    sample_rate: u64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtmpVideo {
    codec: String,
    width: u64,
    height: u64,
    frame_rate: f64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RtmpClient {
    id: u64,
    address: String,
    connection_time: String,
    flash_version: String,
    #[serde(rename = "swfURL")]
    swf_url: String,
    dropped_packets: u64,
    av_sync: u64,
    timestamp: u64,
    publishing: bool,
    active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Irlhosting {
    /// URL to the IRLHosting stats page (ex; http://127.0.0.1:8181/stats )
    pub stats_url: String,

    /// Stream application name (ex; publish)
    pub application: Option<String>,

    /// Stream key (ex; live)
    pub key: Option<String>,

    /// StreamID of where you are publishing the feed. (ex; publish/live/feed1 )
    pub publisher: Option<String>,

    /// Client to make HTTP requests with
    #[serde(skip, default = "default_reqwest_client")]
    pub client: reqwest::Client,
}

impl Irlhosting {
    pub async fn get_stats(&self) -> Option<Stat> {
        let res = self
            .client
            .get(&self.stats_url)
            .send()
            .await
            .map_err(|e| {
                error!("Stats page is unreachable, {}", e);
            })
            .ok()?;

        if res.status() != reqwest::StatusCode::OK {
            error!("Error accessing stats page ({})", self.stats_url);
            return None;
        }

        let stream: Stat = res
            .json()
            .await
            .map_err(|e| {
                error!("Error parsing stats ({}) {}", self.stats_url, e);
            })
            .ok()?;

        trace!("{:#?}", stream);
        Some(stream)
    }

    pub fn get_rtmp_stream<'a>(
        &self,
        applications: &'a [RtmpApplication],
    ) -> Option<&'a RtmpStream> {
        let application = self.application.as_ref()?;
        let key = self.key.as_ref()?;

        let application = applications.iter().find(|a| a.name == *application)?;
        let stream = application.live.streams.iter().find(|s| s.name == *key)?;

        Some(stream)
    }

    pub fn get_srt_publisher<'a>(
        &self,
        publishers: &'a [SrtPublisher],
    ) -> Option<&'a SrtPublisher> {
        let publisher = self.publisher.as_ref()?;

        let publisher = publishers.iter().find(|s| s.stream == *publisher)?;

        Some(publisher)
    }
}

async fn switch_rtmp(triggers: &Triggers, stats: &RtmpStream) -> SwitchType {
    let bitrate = stats.bytes.incoming / 1024;

    if let Some(offline) = triggers.offline {
        if bitrate > 0 && bitrate <= offline {
            return SwitchType::Offline;
        }
    }

    if bitrate == 0 {
        return SwitchType::Previous;
    }

    if let Some(low) = triggers.low {
        if bitrate <= low {
            return SwitchType::Low;
        }
    }

    SwitchType::Normal
}

async fn switch_srt(triggers: &Triggers, stats: &SrtPublisher) -> SwitchType {
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

    SwitchType::Normal
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Irlhosting {
    /// Which scene to switch to
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let Some(stats) = self.get_stats().await else {
            return SwitchType::Offline;
        };

        match stats {
            Stat::Srt { publishers, .. } => {
                let Some(publisher) = self.get_srt_publisher(&publishers) else {
                    return SwitchType::Offline;
                };

                switch_srt(triggers, publisher).await
            }
            Stat::Rtmp { applications, .. } => {
                let Some(stream) = self.get_rtmp_stream(&applications) else {
                    return SwitchType::Offline;
                };

                switch_rtmp(triggers, stream).await
            }
        }
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Irlhosting {
    async fn bitrate(&self) -> super::Bitrate {
        let Some(stats) = self.get_stats().await else {
            return super::Bitrate { message: None };
        };

        match stats {
            Stat::Srt { publishers, .. } => {
                let Some(publisher) = self.get_srt_publisher(&publishers) else {
                    return super::Bitrate { message: None };
                };

                let message = format!("{}, {} ms", publisher.bitrate, publisher.rtt.round());
                super::Bitrate {
                    message: Some(message),
                }
            }
            Stat::Rtmp { applications, .. } => {
                let Some(stats) = self.get_rtmp_stream(&applications) else {
                    return super::Bitrate { message: None };
                };

                let bitrate = stats.bytes.incoming / 1024;
                super::Bitrate {
                    message: Some(format!("{}", bitrate)),
                }
            }
        }
    }

    async fn source_info(&self) -> Option<String> {
        let stats = self.get_stats().await?;

        match stats {
            Stat::Srt { publishers, .. } => {
                let stats = self.get_srt_publisher(&publishers)?;
                let bitrate = format!("{} Kbps, {} ms", stats.bitrate, stats.rtt.round());

                let mbps = format!(
                    "Estimated bandwidth {} Mbps, Receiving rate {:.2} Mbps",
                    stats.mbps_bandwidth.round(),
                    stats.mbps_recv_rate
                );

                let pkt = format!(
                    "{} dropped, {} loss",
                    stats.packet_stats.packets_dropped, stats.packet_stats.packets_lost
                );

                let ms_buf = format!("{} ms buffer", stats.buffer);

                Some(format!("{} | {} | {} |  {}", bitrate, mbps, pkt, ms_buf))
            }
            Stat::Rtmp { applications, .. } => {
                let stats = self.get_rtmp_stream(&applications)?;

                let video = &stats.video;
                let audio = &stats.audio;

                let bitrate = format!("{} Kbps", stats.bytes.incoming / 1024);

                let v_info = format!("{}p{} | {}", video.height, video.frame_rate, video.codec,);

                let a_info = format!(
                    "{} {} {}Hz, {} channels",
                    audio.codec, audio.profile, audio.sample_rate, audio.channels,
                );

                Some(format!("{} | {} | {}", bitrate, v_info, a_info))
            }
        }
    }
}

#[typetag::serde]
impl Bsl for Irlhosting {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srt() {
        let s = r#"{"service":"SRT","status":"ok","timestamp":"2024-10-05T04:45:34.035Z","publishers":[{"stream":"publish/live/feed1","bitrate":8438,"bitrate_mbps":"0.01","bytesReceived":0,"bytesDropped":0,"bytesLost":0,"mbpsBandwidth":2890.92,"mbpsRecvRate":7.585321421472997,"rtt":3.56,"uptime":"0d 0h 0m 3s","packetStats":{"packetsDropped":0,"packetsLost":0},"buffer":"101ms"}],"overall":{"totalPublishers":1,"totalBytesReceived":0,"totalBytesDropped":0,"totalBytesLost":0,"averageBitrate":"0.01","totalUptime":"0d 0h 0m 3s"}}"#;
        let parsed: Stat = serde_json::from_str(s).unwrap();
        println!("{:#?}", parsed);

        // assert!(parsed.receiver_stats.is_none());
    }

    #[test]
    fn test_rtmp() {
        let s = r#"{"service":"RTMP","server":{"nginxVersion":"1.24.0","rtmpVersion":"1.1.4","compiler":"gcc 11.4.0 (Ubuntu 11.4.0-1ubuntu1~22.04) ","built":"Jun 11 2024 04:25:11","pid":"506","uptime":"0d 0h 1m 48s","connections":{"accepted":1,"current":1},"bandwidth":{"incoming":"0","outgoing":"0"},"bytes":{"incoming":714293,"outgoing":409}},"applications":[{"name":"publish","live":{"streams":[{"name":"live","duration":"0d 0h 18m 46s","bandwidth":{"incoming":"0","outgoing":"0"},"bytes":{"incoming":712916,"outgoing":0},"audio":{"codec":"AAC","profile":"LC","channels":2,"sampleRate":44100},"video":{"codec":"Jpeg","width":1920,"height":1080,"frameRate":30},"clients":[{"id":5,"address":"143.189.206.175","connectionTime":"0d 0h 21m 21s","flashVersion":"FMLE/3.0 (compatible; FMSc/1.0)","swfURL":"rtmp://#####:1935/publish","droppedPackets":0,"avSync":2,"timestamp":668,"publishing":true,"active":true}],"status":"Active"}],"totalClients":1}}],"timestamp":"2024-10-05T04:44:26.646Z"}"#;
        let parsed: Stat = serde_json::from_str(s).unwrap();
        println!("{:#?}", parsed);

        // assert!(parsed.receiver_stats.is_none());
    }
}
