use std::{
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    sync::{Notify, RwLock},
    task::JoinHandle,
    time::{Instant, timeout},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, trace};

use super::{Bitrate, Bsl, StreamServersCommands, SwitchLogic};
use crate::switcher::{SwitchType, Triggers};

const DEFAULT_RECONNECT_INTERVAL_MS: u64 = 1000;
const DEFAULT_STALE_TIMEOUT_MS: u64 = 3000;
const MIN_RECONNECT_INTERVAL_MS: u64 = 250;
const MIN_STALE_TIMEOUT_MS: u64 = 250;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const MIN_IDLE_TIMEOUT: Duration = Duration::from_secs(10);

static STATS_UPDATED: OnceLock<Arc<Notify>> = OnceLock::new();

pub fn stats_update_notifier() -> Arc<Notify> {
    STATS_UPDATED
        .get_or_init(|| Arc::new(Notify::new()))
        .clone()
}

fn notify_stats_updated() {
    stats_update_notifier().notify_waiters();
}

#[derive(Clone, Debug)]
struct CachedStats {
    stream_id: Option<String>,
    feed: Option<String>,
    bitrate: u64,
    packet_loss: Option<f64>,
    rtt: Option<f64>,
    connected: bool,
    received_at: Instant,
}

#[derive(Debug, Default)]
struct WebSocketStatsInner {
    latest: RwLock<Option<CachedStats>>,
    stale_logged: AtomicBool,
}

impl WebSocketStatsInner {
    async fn latest(&self) -> Option<CachedStats> {
        self.latest.read().await.clone()
    }

    async fn update(&self, stats: CachedStats) {
        *self.latest.write().await = Some(stats);
        self.stale_logged.store(false, Ordering::Relaxed);
        notify_stats_updated();
    }

    fn log_stale_once(&self, stale_timeout: Duration) {
        if self.stale_logged.swap(true, Ordering::Relaxed) {
            return;
        }

        info!(
            "[WebSocketStats] No stats received for {}ms, marking stale",
            stale_timeout.as_millis()
        );
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketStats {
    pub url: String,
    pub feed: Option<String>,
    pub token: Option<String>,
    pub reconnect_interval_ms: Option<u64>,
    pub stale_timeout_ms: Option<u64>,

    #[serde(skip, default = "default_inner")]
    inner: Arc<WebSocketStatsInner>,

    #[serde(skip, default = "default_task")]
    task: std::sync::Mutex<Option<JoinHandle<()>>>,
}

fn default_inner() -> Arc<WebSocketStatsInner> {
    Arc::new(WebSocketStatsInner::default())
}

fn default_task() -> std::sync::Mutex<Option<JoinHandle<()>>> {
    std::sync::Mutex::new(None)
}

impl WebSocketStats {
    async fn latest_stats(&self) -> Option<CachedStats> {
        self.ensure_connected();

        let stats = self.inner.latest().await?;
        let stale_timeout = self.stale_timeout();

        if stats.received_at.elapsed() > stale_timeout {
            self.inner.log_stale_once(stale_timeout);
            return None;
        }

        Some(stats)
    }

    fn ensure_connected(&self) {
        let Ok(mut task) = self.task.lock() else {
            return;
        };

        if task.as_ref().is_some_and(|task| !task.is_finished()) {
            return;
        }

        let url = self.url_with_token();
        let log_url = sanitized_url(&url);
        let feed = self.configured_feed();
        let config = ClientConfig {
            reconnect_interval: self.reconnect_interval(),
            idle_timeout: self.idle_timeout(),
        };
        let inner = self.inner.clone();

        *task = Some(tokio::spawn(async move {
            run_websocket_client(url, log_url, feed, config, inner).await;
        }));
    }

    fn configured_feed(&self) -> Option<String> {
        self.feed
            .clone()
            .or_else(|| query_value(&self.url, "feed"))
            .filter(|feed| !feed.is_empty())
    }

    fn url_with_token(&self) -> String {
        let Some(token) = &self.token else {
            return self.url.clone();
        };

        if token.is_empty() || query_value(&self.url, "token").is_some() {
            return self.url.clone();
        }

        let Ok(mut url) = reqwest::Url::parse(&self.url) else {
            return self.url.clone();
        };

        url.query_pairs_mut().append_pair("token", token);
        url.to_string()
    }

    fn reconnect_interval(&self) -> Duration {
        Duration::from_millis(
            self.reconnect_interval_ms
                .unwrap_or(DEFAULT_RECONNECT_INTERVAL_MS)
                .max(MIN_RECONNECT_INTERVAL_MS),
        )
    }

    fn stale_timeout(&self) -> Duration {
        Duration::from_millis(
            self.stale_timeout_ms
                .unwrap_or(DEFAULT_STALE_TIMEOUT_MS)
                .max(MIN_STALE_TIMEOUT_MS),
        )
    }

    fn idle_timeout(&self) -> Duration {
        self.stale_timeout().mul_f32(3.0).max(MIN_IDLE_TIMEOUT)
    }
}

#[derive(Clone, Copy, Debug)]
struct ClientConfig {
    reconnect_interval: Duration,
    idle_timeout: Duration,
}

impl Drop for WebSocketStats {
    fn drop(&mut self) {
        if let Ok(mut task) = self.task.lock()
            && let Some(task) = task.take()
        {
            task.abort();
        }
    }
}

async fn run_websocket_client(
    url: String,
    log_url: String,
    feed: Option<String>,
    config: ClientConfig,
    inner: Arc<WebSocketStatsInner>,
) {
    loop {
        info!(
            "[WebSocketStats] Connecting to {} feed={}",
            log_url,
            feed.as_deref().unwrap_or("<first-active>")
        );

        match timeout(CONNECT_TIMEOUT, connect_async(&url)).await {
            Ok(Ok((mut ws, _))) => {
                info!("[WebSocketStats] Connected");

                loop {
                    match timeout(config.idle_timeout, ws.next()).await {
                        Ok(Some(Ok(Message::Text(text)))) => {
                            handle_message(&text, feed.as_deref(), &inner).await;
                        }
                        Ok(Some(Ok(Message::Binary(bytes)))) => match String::from_utf8(bytes) {
                            Ok(text) => handle_message(&text, feed.as_deref(), &inner).await,
                            Err(error) => {
                                trace!("[WebSocketStats] Non-UTF8 binary message: {}", error)
                            }
                        },
                        Ok(Some(Ok(Message::Close(_)))) | Ok(None) => break,
                        Ok(Some(Ok(_))) => {}
                        Ok(Some(Err(error))) => {
                            error!("[WebSocketStats] WebSocket error: {}", error);
                            break;
                        }
                        Err(_) => {
                            info!(
                                "[WebSocketStats] No WebSocket message received for {}ms, reconnecting",
                                config.idle_timeout.as_millis()
                            );
                            break;
                        }
                    }
                }
            }
            Ok(Err(error)) => {
                error!("[WebSocketStats] Connection failed: {}", error);
            }
            Err(_) => {
                error!(
                    "[WebSocketStats] Connection timed out after {}ms",
                    CONNECT_TIMEOUT.as_millis()
                );
            }
        }

        info!(
            "[WebSocketStats] Disconnected, reconnecting in {}ms",
            config.reconnect_interval.as_millis()
        );
        tokio::time::sleep(config.reconnect_interval).await;
    }
}

async fn handle_message(text: &str, feed: Option<&str>, inner: &Arc<WebSocketStatsInner>) {
    match parse_stats_message(text, feed) {
        Some(stats) => {
            debug!(
                "[WebSocketStats] Stats update feed={} bitrate={}",
                stats.feed.as_deref().unwrap_or("<unknown>"),
                stats.bitrate
            );

            inner.update(stats).await;
        }
        None => trace!("[WebSocketStats] Ignoring unsupported or unmatched message"),
    }
}

fn parse_stats_message(text: &str, feed: Option<&str>) -> Option<CachedStats> {
    let value: Value = serde_json::from_str(text).ok()?;

    match value.get("type").and_then(Value::as_str) {
        Some("stats") => parse_normalized_stats(&value, feed),
        Some("sls_stats_xml") => None,
        _ => None,
    }
}

fn parse_normalized_stats(value: &Value, feed: Option<&str>) -> Option<CachedStats> {
    if let Some(streams) = value.get("streams").and_then(Value::as_array) {
        return select_stream_stats(streams, feed).or_else(|| feed.map(offline_stats_for_feed));
    }

    parse_stream_stats(value).filter(|stats| matches_feed(stats, feed))
}

fn offline_stats_for_feed(feed: &str) -> CachedStats {
    CachedStats {
        stream_id: Some(feed.to_string()),
        feed: Some(feed.to_string()),
        bitrate: 0,
        packet_loss: None,
        rtt: None,
        connected: false,
        received_at: Instant::now(),
    }
}

fn select_stream_stats(streams: &[Value], feed: Option<&str>) -> Option<CachedStats> {
    let stats = streams
        .iter()
        .filter_map(parse_stream_stats)
        .collect::<Vec<_>>();

    if feed.is_some() {
        return stats.into_iter().find(|stats| matches_feed(stats, feed));
    }

    stats
        .iter()
        .find(|stats| stats.connected && stats.bitrate > 0)
        .cloned()
        .or_else(|| stats.into_iter().next())
}

fn parse_stream_stats(value: &Value) -> Option<CachedStats> {
    let bitrate = value.get("bitrate")?.as_u64()?;
    let stream_id = value
        .get("streamId")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let feed = value
        .get("feed")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| {
            stream_id
                .as_deref()
                .and_then(feed_from_stream_id)
                .map(ToOwned::to_owned)
        });
    let packet_loss = value.get("packetLoss").and_then(Value::as_f64);
    let rtt = value.get("rtt").and_then(Value::as_f64);
    let connected = value
        .get("connected")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    Some(CachedStats {
        stream_id,
        feed,
        bitrate,
        packet_loss,
        rtt,
        connected,
        received_at: Instant::now(),
    })
}

fn matches_feed(stats: &CachedStats, feed: Option<&str>) -> bool {
    let Some(feed) = feed else {
        return true;
    };

    stats.feed.as_deref() == Some(feed)
        || stats.stream_id.as_deref() == Some(feed)
        || stats
            .stream_id
            .as_deref()
            .and_then(feed_from_stream_id)
            .is_some_and(|last| last == feed)
}

fn feed_from_stream_id(stream_id: &str) -> Option<&str> {
    stream_id.rsplit('/').next().filter(|feed| !feed.is_empty())
}

fn query_value(url: &str, key: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()?
        .query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

fn sanitized_url(url: &str) -> String {
    let Ok(mut parsed) = reqwest::Url::parse(url) else {
        return url.to_string();
    };

    let pairs = parsed
        .query_pairs()
        .filter_map(|(key, value)| {
            if key == "token" {
                None
            } else {
                Some((key.into_owned(), value.into_owned()))
            }
        })
        .collect::<Vec<_>>();

    parsed.set_query(None);

    if !pairs.is_empty() {
        let mut query = parsed.query_pairs_mut();
        for (key, value) in pairs {
            query.append_pair(&key, &value);
        }
    }

    parsed.to_string()
}

fn switch_from_stats(stats: &CachedStats, triggers: &Triggers) -> SwitchType {
    if !stats.connected {
        return SwitchType::Offline;
    }

    if let Some(offline) = triggers.offline
        && stats.bitrate > 0
        && stats.bitrate <= offline.into()
    {
        return SwitchType::Offline;
    }

    if let Some(rtt_offline) = triggers.rtt_offline
        && stats.rtt.is_some_and(|rtt| rtt >= rtt_offline.into())
    {
        return SwitchType::Offline;
    }

    if stats.bitrate == 0 {
        return SwitchType::Previous;
    }

    if let Some(low) = triggers.low
        && stats.bitrate <= low.into()
    {
        return SwitchType::Low;
    }

    if let Some(rtt) = triggers.rtt
        && stats.rtt.is_some_and(|stat_rtt| stat_rtt >= rtt.into())
    {
        return SwitchType::Low;
    }

    SwitchType::Normal
}

#[async_trait]
#[typetag::serde(name = "WebSocket")]
impl SwitchLogic for WebSocketStats {
    async fn switch(&self, triggers: &Triggers) -> SwitchType {
        let Some(stats) = self.latest_stats().await else {
            return SwitchType::Offline;
        };

        switch_from_stats(&stats, triggers)
    }
}

#[async_trait]
#[typetag::serde(name = "WebSocket")]
impl StreamServersCommands for WebSocketStats {
    async fn bitrate(&self) -> Bitrate {
        let Some(stats) = self.latest_stats().await else {
            return Bitrate { message: None };
        };

        let message = match stats.rtt {
            Some(rtt) => format!("{}, {} ms", stats.bitrate, rtt.round()),
            None => format!("{}", stats.bitrate),
        };

        Bitrate {
            message: Some(message),
        }
    }

    async fn source_info(&self) -> Option<String> {
        let stats = self.latest_stats().await?;
        let feed = stats.feed.as_deref().unwrap_or("unknown");
        let bitrate = format!("{} Kbps", stats.bitrate);
        let rtt = stats
            .rtt
            .map(|rtt| format!("{} ms", rtt.round()))
            .unwrap_or_else(|| "unknown rtt".to_string());
        let packet_loss = stats
            .packet_loss
            .map(|packet_loss| format!("{} packet loss", packet_loss))
            .unwrap_or_else(|| "unknown packet loss".to_string());

        Some(format!(
            "{} | {} | {} | feed {}",
            bitrate, rtt, packet_loss, feed
        ))
    }
}

#[typetag::serde(name = "WebSocket")]
impl Bsl for WebSocketStats {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn instant_degrade(&self) -> bool {
        true
    }

    fn delay_normal_recovery(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_feed_stats() {
        let text = r#"{"type":"stats","timestamp":1710000000000,"streamId":"publish/live/feed2","bitrate":6200,"packetLoss":0,"rtt":80,"connected":true}"#;
        let stats = parse_stats_message(text, Some("feed2")).unwrap();

        assert_eq!(stats.feed.as_deref(), Some("feed2"));
        assert_eq!(stats.bitrate, 6200);
        assert_eq!(stats.rtt, Some(80.0));
        assert!(stats.connected);
    }

    #[test]
    fn filters_multi_stream_by_feed() {
        let text = r#"{"type":"stats","timestamp":1710000000000,"streams":[{"streamId":"publish/live/feed1","feed":"feed1","bitrate":300,"connected":true},{"streamId":"publish/live/cameraA","feed":"cameraA","bitrate":6200,"connected":true}]}"#;
        let stats = parse_stats_message(text, Some("cameraA")).unwrap();

        assert_eq!(stats.feed.as_deref(), Some("cameraA"));
        assert_eq!(stats.bitrate, 6200);
    }

    #[test]
    fn selects_first_active_stream_without_feed() {
        let text = r#"{"type":"stats","timestamp":1710000000000,"streams":[{"streamId":"publish/live/feed1","bitrate":0,"connected":true},{"streamId":"publish/live/feed2","bitrate":6200,"connected":true}]}"#;
        let stats = parse_stats_message(text, None).unwrap();

        assert_eq!(stats.feed.as_deref(), Some("feed2"));
        assert_eq!(stats.bitrate, 6200);
    }

    #[test]
    fn missing_configured_feed_maps_offline() {
        let text = r#"{"type":"stats","timestamp":1710000000000,"streams":[]}"#;
        let stats = parse_stats_message(text, Some("feed1")).unwrap();

        assert_eq!(stats.feed.as_deref(), Some("feed1"));
        assert_eq!(stats.bitrate, 0);
        assert!(!stats.connected);
    }

    #[test]
    fn sanitizes_token_from_url() {
        let url = sanitized_url("ws://127.0.0.1/ws-stats?token=SECRET&feed=feed1");

        assert_eq!(url, "ws://127.0.0.1/ws-stats?feed=feed1");
    }

    #[test]
    fn maps_low_and_offline_states() {
        let triggers = Triggers {
            low: Some(800),
            rtt: Some(1500),
            offline: Some(400),
            rtt_offline: None,
        };
        let mut stats = CachedStats {
            stream_id: Some("publish/live/feed1".to_string()),
            feed: Some("feed1".to_string()),
            bitrate: 300,
            packet_loss: None,
            rtt: Some(80.0),
            connected: true,
            received_at: Instant::now(),
        };

        assert_eq!(switch_from_stats(&stats, &triggers), SwitchType::Offline);

        stats.bitrate = 600;
        assert_eq!(switch_from_stats(&stats, &triggers), SwitchType::Low);

        stats.bitrate = 6000;
        assert_eq!(switch_from_stats(&stats, &triggers), SwitchType::Normal);
    }

    #[test]
    fn deserializes_stream_server_config() {
        let text = r#"{"streamServer":{"type":"WebSocket","url":"ws://127.0.0.1/ws-stats?feed=feed1","reconnectIntervalMs":1000,"staleTimeoutMs":3000},"name":"websocket","priority":0,"overrideScenes":null,"dependsOn":null,"enabled":true}"#;
        let mut server: crate::stream_servers::StreamServer = serde_json::from_str(text).unwrap();

        assert_eq!(server.name, "websocket");
        assert!(
            server
                .stream_server
                .as_any_mut()
                .downcast_mut::<WebSocketStats>()
                .is_some()
        );
    }
}
