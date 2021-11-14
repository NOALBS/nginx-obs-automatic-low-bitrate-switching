use async_trait::async_trait;
use log::{error, trace};
use serde::{Deserialize, Serialize};

use super::{Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

#[derive(Deserialize, Debug)]
struct NginxRtmpStats {
    server: NginxRtmpServer,
}

#[derive(Deserialize, Debug)]
struct NginxRtmpServer {
    application: Vec<NginxRtmpApp>,
}

#[derive(Deserialize, Debug)]
struct NginxRtmpApp {
    name: String,
    live: NginxRtmpLive,
}

#[derive(Deserialize, Debug)]
struct NginxRtmpLive {
    stream: Option<Vec<NginxRtmpStream>>,
}

#[derive(Deserialize, Debug)]
pub struct NginxRtmpStream {
    pub name: String,
    pub bw_video: u32,
    pub meta: Option<Meta>,
}

#[derive(Deserialize, Debug)]
pub struct Meta {
    video: Video,
    audio: Audio,
}

#[derive(Deserialize, Debug)]
pub struct Video {
    width: u32,
    height: u32,
    frame_rate: f64,
    codec: String,
    profile: Option<String>,
    compat: Option<u32>,
    level: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct Audio {
    codec: Option<String>,
    profile: Option<String>,
    channels: Option<u32>,
    sample_rate: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Nginx {
    /// Url to the NGINX stats page
    pub stats_url: String,

    /// Stream application
    pub application: String,

    /// Stream key
    pub key: String,
}

impl Nginx {
    /// 0 bitrate means the stream just started.
    /// the stats update every 10 seconds.
    pub async fn get_stats(&self) -> Option<NginxRtmpStream> {
        //TODO: keep the reqwest object around for future requests
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
        let parsed: NginxRtmpStats = match quick_xml::de::from_str(&text) {
            Ok(stats) => stats,
            Err(error) => {
                trace!("{}", &text);
                error!("Error parsing stats ({}) {}", self.stats_url, error);
                return None;
            }
        };

        let filter: Option<NginxRtmpStream> = parsed
            .server
            .application
            .into_iter()
            .filter_map(|x| {
                if x.name == self.application {
                    x.live.stream
                } else {
                    None
                }
            })
            .flatten()
            .filter(|x| x.name == self.key)
            .collect::<Vec<NginxRtmpStream>>()
            .pop();

        trace!("{:#?}", filter);
        filter
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Nginx {
    /// Which scene to switch to
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let stats = match self.get_stats().await {
            Some(b) => b,
            None => return SwitchType::Offline,
        };

        let bitrate = stats.bw_video / 1024;

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

        return SwitchType::Normal;
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Nginx {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = match self.get_stats().await {
            Some(stats) => stats,
            None => return super::Bitrate { message: None },
        };

        let bitrate = stats.bw_video / 1024;
        super::Bitrate {
            message: Some(format!("{}", bitrate)),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let stats = self.get_stats().await?;
        let meta = stats.meta?;
        let video = meta.video;
        let audio = meta.audio;

        let bitrate = format!("{} Kbps", stats.bw_video / 1024);

        let v_info = format!(
            "{}p{} | {} {} {}",
            video.height,
            video.frame_rate,
            video.codec,
            video.profile.unwrap_or_default(),
            video.level.unwrap_or_default()
        );

        let a_info = format!(
            "{} {} {}Hz, {} channels",
            audio.codec.unwrap_or_default(),
            audio.profile.unwrap_or_default(),
            audio.sample_rate.unwrap_or_default(),
            audio.channels.unwrap_or_default(),
        );

        Some(format!("{} | {} | {}", bitrate, v_info, a_info))
    }
}

#[typetag::serde]
impl Bsl for Nginx {}

// impl From<db::StreamServer> for Nginx {
//     fn from(item: db::StreamServer) -> Self {
//         Self {
//             stats_url: item.stats_url,
//             application: item.application,
//             key: item.key,
//             name: item.name,
//             priority: item.priority,
//         }
//     }
// }
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_digit() {
        let text = r#"
            <?xml version="1.0" encoding="utf-8" ?>
            <?xml-stylesheet type="text/xsl" href="/stat.xsl" ?>
            <rtmp>
                <nginx_version>1.17.6</nginx_version>
                <nginx_rtmp_version>1.1.7.11-dev</nginx_rtmp_version>
                <compiler>gcc 9.3.0 (Ubuntu 9.3.0-17ubuntu1~20.04) </compiler>
                <built>Feb 15 2021 01:06:01</built>
                <pid>944</pid>
                <uptime>1781693</uptime>
                <naccepted>262</naccepted>
                <bw_in>0</bw_in>
                <bytes_in>1185796040</bytes_in>
                <bw_out>0</bw_out>
                <bytes_out>646442932</bytes_out>
                <server>
                    <application>
                        <name>publish</name>
                        <live>
                            <stream>
                                <name>test</name>
                                <time>1832</time>
                                <bw_in>0</bw_in>
                                <bytes_in>766897</bytes_in>
                                <bw_out>0</bw_out>
                                <bytes_out>0</bytes_out>
                                <bw_audio>0</bw_audio>
                                <bw_video>0</bw_video>
                                <bw_data>0</bw_data>
                                <client>
                                    <id>3192</id>
                                    <address>123.123.123.123</address>
                                    <port>59796</port>
                                    <time>2008</time>
                                    <flashver>FMLE/3.0 (compatible; Larix/nul</flashver>
                                    <bytes_in>768181</bytes_in>
                                    <bytes_out>409</bytes_out>
                                    <dropped>0</dropped>
                                    <avsync></avsync>
                                    <timestamp>3361</timestamp>
                                    <publishing/>
                                    <active/>
                                </client>
                                <meta>
                                    <video>
                                        <width>1280</width>
                                        <height>720</height>
                                        <frame_rate>0.000</frame_rate>
                                        <codec>H264</codec>
                                        <profile>Baseline</profile>
                                        <compat>192</compat>
                                        <level>3.1</level>
                                    </video>
                                    <audio></audio>
                                </meta>
                                <nclients>1</nclients>
                                <publishing/>
                                <active/>
                            </stream>
                            <nclients>1</nclients>
                        </live>
                    </application>
                </server>
            </rtmp>
        "#;

        let parsed: NginxRtmpStats = quick_xml::de::from_str(&text).unwrap();
        println!("{:#?}", parsed);
    }
}
