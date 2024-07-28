use twilight_gateway::ShardId;

use crate::Bot;

#[derive(Debug, Clone)]
pub struct ShardContext {
    pub bot: Bot,
    pub shard_id: ShardId,
}
