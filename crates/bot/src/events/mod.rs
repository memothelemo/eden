use eden_utils::Result;
use twilight_gateway::Event;

use crate::shard::ShardContext;

// TODO: Allow individual shards to alert the central thread that they successfully connected to the gateway
#[tracing::instrument(skip_all, fields(
    guild.id = ?event.guild_id(),
    kind = ?event.kind(),
    shard.id = ?ctx.shard_id,
))]
pub async fn handle_event(ctx: ShardContext, event: Event) -> Result<()> {
    match event {
        Event::Ready(..) => {
            tracing::debug!("shard is ready");
        }
        Event::Resumed => {
            tracing::debug!("shard resumed gateway session");
        }
        _ => {
            tracing::debug!("received event {:?}", event.kind());
        }
    }
    Ok(())
}
