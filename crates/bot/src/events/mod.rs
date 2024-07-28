use eden_utils::Result;
use twilight_gateway::Event;

use crate::shard::ShardContext;

pub mod ready;

// TODO: Allow individual shards to alert the central thread that they successfully connected to the gateway
#[tracing::instrument(skip_all, fields(
    ctx.latency = ?ctx.recent_latency(),
    guild.id = ?event.guild_id(),
    event.kind = ?event.kind(),
    shard.id = %ctx.shard_id,
))]
pub async fn handle_event(ctx: ShardContext, event: Event) {
    let result: Result<()> = match &event {
        Event::Ready(data) => self::ready::handle(&ctx, &data),
        Event::Resumed => {
            tracing::debug!("shard resumed gateway session");
            Ok(())
        }
        _ => {
            tracing::debug!("received event {:?}", event.kind());
            Ok(())
        }
    };

    if let Err(error) = result {
        tracing::warn!(%error, "unhandled error from event {:?}", event.kind());
    }
}
