use eden_utils::Result;
use twilight_gateway::{Event, EventTypeFlags, Intents};

use crate::shard::ShardContext;

pub mod interaction_create;
pub mod ready;

pub(crate) const INTENTS: Intents = Intents::GUILDS
    .union(Intents::DIRECT_MESSAGES)
    .union(Intents::GUILD_MESSAGES);

pub(crate) const FILTERED_EVENT_TYPES: EventTypeFlags = EventTypeFlags::READY
    .union(EventTypeFlags::RESUMED)
    .union(EventTypeFlags::INTERACTION_CREATE)
    .union(EventTypeFlags::DIRECT_MESSAGES);

#[tracing::instrument(skip_all, fields(
    ctx.latency = ?ctx.recent_latency(),
    guild.id = ?event.guild_id(),
    event.kind = ?event.kind(),
    shard.id = %ctx.shard_id,
))]
pub async fn handle_event(ctx: ShardContext, event: Event) {
    let event_kind = event.kind();
    let result: Result<()> = match event {
        Event::InteractionCreate(data) => self::interaction_create::handle(&ctx, *data).await,
        Event::Ready(data) => self::ready::handle(&ctx, &data).await,
        Event::Resumed => {
            tracing::debug!("shard resumed gateway session");
            Ok(())
        }
        _ => {
            tracing::warn!("received unimplemented {event_kind:?} event");
            Ok(())
        }
    };

    if let Err(error) = result {
        tracing::warn!(%error, "unhandled error from event {event_kind:?}");
    }
}
