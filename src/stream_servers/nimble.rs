use crate::stream_servers::SwitchLogic;
use async_trait::async_trait;

use super::Triggers;

pub struct Nimble {
    /// UDP listener ID (Usually IP:Port)
    pub id: String,

    /// URL to nimble API
    pub stats_url: String,

    /// Outgoing stream "Application Name"
    pub application: String,

    /// Outgoing stream "Stream Name"
    pub key: String,

    /// Lowbitrate trigger
    pub trigger: u64,

    /// HighRtt trigger
    pub rtt_trigger: u64,
}

impl Nimble {
    pub async fn get_stats(&self) {
        unimplemented!()
    }
}

#[async_trait]
impl SwitchLogic for Nimble {
    async fn switch(&self, triggers: &Triggers) -> super::SwitchType {
        todo!()
    }
}
