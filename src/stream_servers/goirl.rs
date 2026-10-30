use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpStream,
    sync::RwLock,
    task::JoinHandle,
    time::{self, Instant},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message as TMessage};
use tracing::{debug, error, info, trace, warn};

use super::{Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

/// go-irl only sends stats while it's receiving packets,
/// no stats for this long means the stream is offline
const STALE_TIMEOUT: Duration = Duration::from_secs(3);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Keep this low since we won't switch back while disconnected
const MAX_RETRY_GROW: u64 = 3;

#[derive(Deserialize, Debug, Clone)]
pub struct Message {
    #[serde(rename = "type")]
    pub kind: MessageKind,
    pub stats: Stat,
    /// SRTLA links, not reported when the stream doesn't come through go-irl's SRTLA receiver
    #[serde(default)]
    pub links: Option<Vec<Link>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    /// Incoming stream
    Reader,
    /// Outgoing stream to OBS
    Writer,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Stat {
    pub accumulated: Accumulated,
    pub instantaneous: Instantaneous,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Accumulated {
    pub pkt_recv_loss: u64,
    pub pkt_recv_drop: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Instantaneous {
    #[serde(rename = "MsRTT")]
    pub ms_rtt: f64,
    pub mbps_recv_rate: f64,
    pub pkt_recv_loss_rate: f64,
    pub ms_recv_buf: u64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: u64,
    pub rx_bytes: u64,
}

impl Stat {
    /// Bitrate in Kbps
    pub fn bitrate(&self) -> u64 {
        (self.instantaneous.mbps_recv_rate * 1000.0).round() as u64
    }

    pub fn rtt(&self) -> f64 {
        self.instantaneous.ms_rtt
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoIrl {
    /// URL to the go-irl WebSocket server (ex; ws://127.0.0.1:8888/ws )
    pub url: String,

    /// Last received reader stats
    #[serde(skip)]
    latest: Arc<RwLock<Option<(Instant, Message)>>>,

    #[serde(skip)]
    connection: Mutex<Option<JoinHandle<()>>>,
}

impl GoIrl {
    pub async fn get_stats(&self) -> Option<Message> {
        self.connect_if_necessary();

        let mut latest = self.latest.write().await;
        let (received_at, message) = latest.clone()?;

        if received_at.elapsed() > STALE_TIMEOUT {
            // Clear so this only gets logged once
            *latest = None;
            info!(
                "No stats received from go-irl for {} seconds",
                STALE_TIMEOUT.as_secs()
            );
            return None;
        }

        trace!("{:#?}", message);
        Some(message)
    }

    /// Stats are pushed by go-irl so keep a connection open in the background
    fn connect_if_necessary(&self) {
        let Ok(mut connection) = self.connection.lock() else {
            return;
        };

        if connection.as_ref().is_some_and(|c| !c.is_finished()) {
            return;
        }

        let url = self.url.clone();
        let latest = self.latest.clone();

        *connection = Some(tokio::spawn(conn_loop(url, latest)));
    }
}

impl Drop for GoIrl {
    fn drop(&mut self) {
        if let Ok(mut connection) = self.connection.lock()
            && let Some(connection) = connection.take()
        {
            connection.abort();
        }
    }
}

async fn conn_loop(url: String, latest: Arc<RwLock<Option<(Instant, Message)>>>) {
    loop {
        let mut connection = get_connection(&url).await;

        while let Some(msg) = connection.next().await {
            match msg {
                Ok(TMessage::Text(text)) => match serde_json::from_str::<Message>(&text) {
                    Ok(message) if message.kind == MessageKind::Reader => {
                        *latest.write().await = Some((Instant::now(), message));
                    }
                    Ok(_) => {}
                    Err(e) => debug!("Error parsing go-irl stats: {}", e),
                },
                Ok(TMessage::Close(_)) => break,
                Ok(_) => {}
                Err(e) => {
                    error!("go-irl WebSocket error: {}", e);
                    break;
                }
            }
        }

        warn!("Disconnected from go-irl ({})", url);
    }
}

async fn get_connection(url: &str) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    let mut retry_grow = 1;

    loop {
        info!("Connecting to go-irl ({})", url);

        match time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url)).await {
            Ok(Ok((ws_stream, _))) => {
                info!("Connected to go-irl");
                break ws_stream;
            }
            Ok(Err(e)) => warn!("Unable to connect to go-irl ({}): {}", url, e),
            Err(_) => warn!("Unable to connect to go-irl ({}): timed out", url),
        }

        let wait = 1 << retry_grow;
        info!("trying to connect again in {} seconds", wait);
        time::sleep(Duration::from_secs(wait)).await;

        if retry_grow < MAX_RETRY_GROW {
            retry_grow += 1;
        }
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for GoIrl {
    /// Which scene to switch to
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let stats = match self.get_stats().await {
            Some(b) => b.stats,
            None => return SwitchType::Offline,
        };

        let bitrate = stats.bitrate();
        let rtt = stats.rtt();

        if let Some(offline) = triggers.offline
            && bitrate > 0
            && bitrate <= offline.into()
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
            && bitrate <= low.into()
        {
            return SwitchType::Low;
        }

        if let Some(trigger) = triggers.rtt
            && rtt >= trigger.into()
        {
            return SwitchType::Low;
        }

        return SwitchType::Normal;
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for GoIrl {
    async fn bitrate(&self) -> super::Bitrate {
        let stats = match self.get_stats().await {
            Some(message) => message.stats,
            None => return super::Bitrate { message: None },
        };

        let message = format!("{}, {} ms", stats.bitrate(), stats.rtt().round());
        super::Bitrate {
            message: Some(message),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let message = self.get_stats().await?;
        let stats = &message.stats;

        let bitrate = format!("{} Kbps, {} ms", stats.bitrate(), stats.rtt().round());

        let pkt = format!(
            "{:.2}% loss, {} lost, {} dropped",
            stats.instantaneous.pkt_recv_loss_rate,
            stats.accumulated.pkt_recv_loss,
            stats.accumulated.pkt_recv_drop
        );

        // The ms of acknowledged packets in the receiver's buffer
        let ms_buf = format!("{} ms buffer", stats.instantaneous.ms_recv_buf);

        let mut info = format!("{} | {} | {}", bitrate, pkt, ms_buf);

        if let Some(links) = &message.links {
            info.push_str(&format!(" | {} links", links.len()));
        }

        Some(info)
    }
}

#[typetag::serde]
impl Bsl for GoIrl {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const READER: &str = r#"{"timestamp":"2026-10-30T12:00:00.000000000Z","type":"reader","stats":{"MsTimeStamp":1000,"Accumulated":{"PktRecv":5000,"PktRecvLoss":12,"PktRecvRetrans":30,"PktRecvDrop":3},"Interval":{"MsInterval":1000},"Instantaneous":{"MsRTT":42.6,"MbpsRecvRate":6.2345,"MbpsLinkCapacity":100,"PktRecvLossRate":0.5,"MsRecvBuf":1980}},"links":[{"id":1,"rxBytes":1000},{"id":2,"rxBytes":2000}]}"#;

    #[test]
    fn parses_reader_stats() {
        let message: Message = serde_json::from_str(READER).unwrap();

        assert_eq!(message.kind, MessageKind::Reader);
        assert_eq!(message.stats.bitrate(), 6235);
        assert_eq!(message.stats.rtt(), 42.6);
        assert_eq!(message.stats.accumulated.pkt_recv_loss, 12);
        assert_eq!(message.stats.accumulated.pkt_recv_drop, 3);
        assert_eq!(message.stats.instantaneous.ms_recv_buf, 1980);
        assert_eq!(message.links.unwrap().len(), 2);
    }

    #[test]
    fn parses_writer_stats_without_links() {
        let text = READER
            .replace(r#""type":"reader""#, r#""type":"writer""#)
            .replace(
                r#","links":[{"id":1,"rxBytes":1000},{"id":2,"rxBytes":2000}]"#,
                "",
            );
        let message: Message = serde_json::from_str(&text).unwrap();

        assert_eq!(message.kind, MessageKind::Writer);
        assert!(message.links.is_none());
    }

    #[test]
    fn deserializes_stream_server_config() {
        let text = r#"{"streamServer":{"type":"GoIrl","url":"ws://127.0.0.1:8888/ws"},"name":"go-irl","priority":0,"overrideScenes":null,"dependsOn":null,"enabled":true}"#;
        let mut server: crate::stream_servers::StreamServer = serde_json::from_str(text).unwrap();

        assert!(
            server
                .stream_server
                .as_any_mut()
                .downcast_mut::<GoIrl>()
                .is_some()
        );
    }
}
