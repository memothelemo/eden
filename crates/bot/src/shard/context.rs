use std::time::Duration;
use twilight_gateway::{Latency, ShardId};

use crate::Bot;

#[derive(Debug, Clone)]
pub struct ShardContext {
    pub bot: Bot,
    pub latency: Latency,
    pub shard_id: ShardId,
}

impl ShardContext {
    #[must_use]
    pub fn recent_latency(&self) -> Duration {
        self.latency
            .recent()
            .first()
            .copied()
            .unwrap_or(Duration::ZERO)
    }
}
