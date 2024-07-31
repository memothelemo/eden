use std::fmt::Debug;
use std::time::Duration;
use twilight_gateway::{Latency, ShardId};

use crate::Bot;

#[derive(Debug, Clone)]
pub struct ShardContext {
    pub bot: Bot,
    pub id: ShardId,
    pub latency: Latency,
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

/// Simplifies the debug structure of ShardContext without
/// showing any redundant data such as the `bot` field.
pub(crate) struct SimplifiedShardContext<'a>(pub &'a ShardContext);

impl<'a> Debug for SimplifiedShardContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShardContext")
            .field("id", &self.0.id)
            .field("latency", &self.0.latency)
            .finish()
    }
}
