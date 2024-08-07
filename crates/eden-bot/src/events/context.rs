use std::time::Duration;
use twilight_gateway::Latency;

use crate::shard::ShardHandle;
use crate::Bot;

#[derive(Debug, Clone)]
pub struct EventContext {
    pub bot: Bot,
    pub latency: Latency,
    pub shard: ShardHandle,
}

impl EventContext {
    #[must_use]
    pub fn get_latency(&self) -> Duration {
        self.latency
            .recent()
            .first()
            .copied()
            .unwrap_or(Duration::ZERO)
    }
}
