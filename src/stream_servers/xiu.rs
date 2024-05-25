use async_trait::async_trait;
use log::{error, trace};
use serde::{Deserialize, Serialize};

use super::{default_reqwest_client, Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
pub struct XiuStreamInfo {
    pub publisher: XiuPublisher,
    pub subscriber_count: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct XiuPublisher {
    pub id: String,
    pub identifier: XiuIdentifier,
    pub start_time: String,
    #[serde(rename = "recv_bitrate(kbits/s)")]
    pub recv_bitrate: u64,
    pub video: Option<XiuVideoInfo>,
    pub audio: Option<XiuAudioInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XiuIdentifier {
    pub rtmp: XiuRtmp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XiuRtmp {
    pub app_name: String,
    pub stream_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XiuVideoInfo {
    #[serde(rename = "bitrate(kbits/s)")]
    pub bitrate: u64,
    pub codec: String,
    pub width: u64,
    pub height: u64,
    pub frame_rate: f64,
    pub gop: u64,
    pub level: String,
    pub profile: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XiuAudioInfo {
    #[serde(rename = "bitrate(kbits/s)")]
    pub bitrate: u64,
    pub channels: u64,
    pub samplerate: u64,
    pub sound_format: String,
    pub profile: String,
}

pub struct XiuConfig {
    /// Url to the stats page
    pub stats_url: String,
    pub application: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Xiu {
    /// Url to the Xiu stats page
    pub stats_url: String,

    /// Stream application
    pub application: String,

    /// Stream key
    pub key: String,

    /// Client to make HTTP requests with
    #[serde(skip, default = "default_reqwest_client")]
    pub client: reqwest::Client,
}

impl Xiu {
    pub async fn get_stats(&self) -> Option<XiuPublisher> {
        let body = serde_json::json!({
            "identifier": {
                "rtmp": {
                    "app_name": self.application,
                    "stream_name": self.key
                }
            }
        });

        let client = &self.client;
        let mut request = client
            .post(&self.stats_url)
            .header("Content-Type", "application/json");

        request = request.json(&body);

        let res = match request.send().await {
            Ok(res) => res,
            Err(_) => {
                error!("Xiu API ({}) is unreachable", self.stats_url);
                return None;
            }
        };

        if res.status() != reqwest::StatusCode::OK {
            error!("Error accessing Xiu API ({})", self.stats_url);
            return None;
        }

        let text = res.text().await.ok()?;
        let data: XiuResponse = serde_json::from_str(&text).ok()?;

        if data.error_code != 0 {
            error!("Error accessing Xiu API ({}) {}", self.stats_url, data.desp);
            return None;
        }

        if data.data.is_empty() {
            error!("No data returned from Xiu API ({})", self.stats_url);
            return None;
        }

        let publisher = serde_json::to_value(&data.data[0].publisher).ok()?;

        let stream: XiuPublisher = match serde_json::from_value(publisher.to_owned()) {
            Ok(stats) => stats,
            Err(error) => {
                trace!("{:?}", &data);
                error!("Error parsing stats ({}) {}", self.stats_url, error);
                return None;
            }
        };

        trace!("{:#?}", stream);
        Some(stream)
    }
}

#[derive(Deserialize, Debug)]
struct XiuResponse {
    error_code: i32,
    desp: String,
    data: Vec<XiuStreamInfo>,
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Xiu {
    /// Which scene to switch to
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let stats = match self.get_stats().await {
            Some(b) => b,
            None => return SwitchType::Offline,
        };

        if let Some(offline) = triggers.offline {
            if stats.recv_bitrate > 0 && stats.recv_bitrate <= offline.into() {
                return SwitchType::Offline;
            }
        }

        if stats.recv_bitrate == 0 {
            return SwitchType::Previous;
        }

        if let Some(low) = triggers.low {
            if stats.recv_bitrate <= low.into() {
                return SwitchType::Low;
            }
        }

        return SwitchType::Normal;
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Xiu {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = match self.get_stats().await {
            Some(stats) => stats,
            None => return super::Bitrate { message: None },
        };

        if stats.video.is_none() {
            return super::Bitrate { message: None };
        }

        let video_bitrate = stats.video.as_ref().unwrap().bitrate;
        super::Bitrate {
            message: Some(format!("{}", video_bitrate)),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let stats = self.get_stats().await?;

        stats.video.as_ref()?;

        let video_info = stats.video.as_ref().unwrap();
        Some(format!(
            "{}x{} {} Kbps, {}",
            video_info.width, video_info.height, video_info.bitrate, video_info.codec
        ))
    }
}

#[typetag::serde]
impl Bsl for Xiu {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_stream() {
        let s = r#"{"error_code":0,"desp":"ok","data":[]}"#;
        let parsed: XiuResponse = serde_json::from_str(s).unwrap();
        println!("{:#?}", parsed);

        assert!(
            parsed.data.is_empty(),
            "There should be no data in the response"
        );
    }

    #[test]
    fn stream() {
        let s = r#"{"error_code":0,"desp":"succ","data":[{"publisher":{"audio":{"bitrate(kbits/s)":128,"channels":2,"profile":"LC","samplerate":44100,"sound_format":"AAC"},"id":"17105458011883","identifier":{"rtmp":{"app_name":"live","stream_name":"source"}},"recv_bitrate(kbits/s)":1948,"remote_address":"127.0.0.1:55764","start_time":"2024-03-16T07:36:41.109177+08:00","video":{"bitrate(kbits/s)":1948,"codec":"H264","frame_rate":20,"gop":60,"height":1280,"level":"3.0","profile":"Main","width":720}},"subscriber_count":2,"subscribers":{"17105458497472":{"id":"17105458497472","remote_address":"127.0.0.1:56450","send_bitrate(kbits/s)":2076,"start_time":"2024-03-16T07:37:29.034025+08:00","sub_type":"PlayerRtmp","total_send_bytes(kbits/s)":74392348},"17105458720121":{"id":"17105458720121","remote_address":"127.0.0.1:56583","send_bitrate(kbits/s)":2076,"start_time":"2024-03-16T07:37:52.999917+08:00","sub_type":"PlayerHttpFlv","total_send_bytes(kbits/s)":69300006}},"total_recv_bytes":91712283,"total_send_bytes":154540637}]}"#;
        let parsed: XiuResponse = serde_json::from_str(s).unwrap();
        assert!(
            parsed.data.len() == 1,
            "There should be one stream in the response"
        );

        let stream = &parsed.data[0];
        assert_eq!(stream.subscriber_count, 2, "Subscriber count should be 2");

        let bitrate = stream.publisher.recv_bitrate;
        assert_eq!(bitrate, 1948, "Bitrate should be 1948");
    }
}
