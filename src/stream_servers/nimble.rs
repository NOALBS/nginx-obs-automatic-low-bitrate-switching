use crate::{db, stream_servers::SwitchLogic};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{Bsl, StreamServersCommands, Triggers};

#[derive(Serialize, Deserialize, Debug)]
pub struct Nimble {
    /// UDP listener ID (Usually IP:Port)
    pub id: String,

    /// URL to nimble API
    pub stats_url: String,

    /// Outgoing stream "Application Name"
    pub application: String,

    /// Outgoing stream "Stream Name"
    pub key: String,

    /// A name to differentiate in case of multiple stream servers
    pub name: String,

    /// Priority
    pub priority: i32,
}

impl Nimble {
    pub async fn get_stats(&self) {
        unimplemented!()
    }
}

#[async_trait]
#[typetag::serde]
impl SwitchLogic for Nimble {
    async fn switch(&self, _triggers: &Triggers) -> super::SwitchType {
        todo!()
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

#[async_trait]
#[typetag::serde]
impl StreamServersCommands for Nimble {
    async fn bitrate(&self) -> super::Bitrate {
        todo!()
    }

    async fn source_info(&self) -> String {
        todo!()
    }
}

#[typetag::serde]
impl Bsl for Nimble {}

impl From<db::StreamServer> for Nimble {
    fn from(item: db::StreamServer) -> Self {
        Self {
            id: item.udp_listener_id,
            stats_url: item.stats_url,
            application: item.application,
            key: item.key,
            name: item.name,
            priority: item.priority,
        }
    }
}
