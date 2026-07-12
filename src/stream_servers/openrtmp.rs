use async_trait::async_trait;
use log::{error, trace};
use serde::{Deserialize, Serialize};

use super::{Bsl, StreamServersCommands, SwitchLogic, default_reqwest_client};
use crate::switcher::{SwitchType, Triggers};

/// Response shape of OpenRTMP's `GET /stats?key=<stats_key>` endpoint.
/// Only present while the stream is live; offline returns a plain-text body.
#[derive(Deserialize, Debug)]
pub struct OpenRTMPStats {
    pub uptime: i64,
    pub bitrate_kbps: f64,
    pub rtt_ms: f64,
    pub bytes_in: u64,
    pub video: Option<OpenRTMPVideo>,
    pub audio: Option<OpenRTMPAudio>,
}

#[derive(Deserialize, Debug)]
pub struct OpenRTMPVideo {
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}

#[derive(Deserialize, Debug)]
pub struct OpenRTMPAudio {
    pub codec: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenRTMP {
    /// Full url to the OpenRTMP stats endpoint, e.g.
    /// `https://host:port/stats?key=<stats_key>`
    pub stats_url: String,

    /// Client to make HTTP requests with
    #[serde(skip, default = "default_reqwest_client")]
    pub client: reqwest::Client,
}

impl OpenRTMP {
    /// The `stats_url` with any query string (which carries the stats key)
    /// stripped, safe to write to logs.
    fn redacted_url(&self) -> &str {
        self.stats_url
            .split_once('?')
            .map_or(self.stats_url.as_str(), |(base, _)| base)
    }

    /// Returns `None` when the stream is offline, unreachable, or the stats
    /// key is invalid.
    pub async fn get_stats(&self) -> Option<OpenRTMPStats> {
        let res = match self.client.get(&self.stats_url).send().await {
            Ok(res) => res,
            Err(_) => {
                error!("OpenRTMP API ({}) is unreachable", self.redacted_url());
                return None;
            }
        };

        if res.status() != reqwest::StatusCode::OK {
            error!("Error accessing OpenRTMP API ({})", self.redacted_url());
            return None;
        }

        let text = res.text().await.ok()?;
        let stats: OpenRTMPStats = match serde_json::from_str(&text) {
            Ok(stats) => stats,
            Err(_) => {
                // Offline streams respond with a plain-text body instead of JSON.
                trace!(
                    "OpenRTMP ({}) stream offline: {}",
                    self.redacted_url(),
                    text
                );
                return None;
            }
        };

        trace!("{:#?}", stats);
        Some(stats)
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for OpenRTMP {
    /// Which scene to switch to
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let stats = match self.get_stats().await {
            Some(s) => s,
            None => return SwitchType::Offline,
        };

        if let Some(offline) = triggers.offline
            && stats.bitrate_kbps > 0.0
            && stats.bitrate_kbps <= offline.into()
        {
            return SwitchType::Offline;
        }

        if let Some(rtt_offline) = triggers.rtt_offline
            && stats.rtt_ms >= rtt_offline.into()
        {
            return SwitchType::Offline;
        }

        if stats.bitrate_kbps == 0.0 {
            return SwitchType::Previous;
        }

        if let Some(low) = triggers.low
            && stats.bitrate_kbps <= low.into()
        {
            return SwitchType::Low;
        }

        if let Some(rtt) = triggers.rtt
            && stats.rtt_ms >= rtt.into()
        {
            return SwitchType::Low;
        }

        SwitchType::Normal
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for OpenRTMP {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = match self.get_stats().await {
            Some(stats) => stats,
            None => return super::Bitrate { message: None },
        };

        super::Bitrate {
            message: Some(format!("{}", stats.bitrate_kbps.round())),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let stats = self.get_stats().await?;
        let video = stats.video.as_ref()?;

        Some(format!(
            "{}x{} {} Kbps, {}",
            video.width,
            video.height,
            stats.bitrate_kbps.round(),
            video.codec
        ))
    }
}

#[typetag::serde]
impl Bsl for OpenRTMP {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream() {
        let s = r#"{"uptime":42,"bitrate_kbps":2500.5,"rtt_ms":18.2,"bytes_in":1048576,"video":{"codec":"H264","width":1920,"height":1080,"fps":60.0},"audio":{"codec":"AAC"}}"#;
        let parsed: OpenRTMPStats = serde_json::from_str(s).unwrap();
        assert_eq!(parsed.bitrate_kbps, 2500.5);
        assert_eq!(parsed.video.unwrap().width, 1920);
        assert_eq!(parsed.audio.unwrap().codec, "AAC");
    }

    #[test]
    fn offline_body_is_not_json() {
        let s = "Stream offline";
        let parsed: Result<OpenRTMPStats, _> = serde_json::from_str(s);
        assert!(parsed.is_err());
    }

    #[test]
    fn redacted_url_strips_stats_key() {
        let s = OpenRTMP {
            stats_url: "http://localhost:8080/stats?key=secret".to_string(),
            client: default_reqwest_client(),
        };
        assert_eq!(s.redacted_url(), "http://localhost:8080/stats");
    }
}
