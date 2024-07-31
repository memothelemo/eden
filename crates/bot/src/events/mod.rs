use eden_utils::Result;
use twilight_cache_inmemory::ResourceType;
use twilight_gateway::{Event, EventTypeFlags, Intents};

use crate::shard::ShardContext;

pub mod guild_create;
pub mod interaction;
pub mod ready;

pub(crate) const SHOULD_CACHE: ResourceType = ResourceType::GUILD
    .union(ResourceType::USER)
    .union(ResourceType::USER_CURRENT)
    .union(ResourceType::CHANNEL);

pub(crate) const INTENTS: Intents = Intents::GUILDS
    .union(Intents::DIRECT_MESSAGES)
    .union(Intents::GUILD_MEMBERS)
    .union(Intents::GUILD_MESSAGES);

pub(crate) const FILTERED_EVENT_TYPES: EventTypeFlags = EventTypeFlags::READY
    .union(EventTypeFlags::RESUMED)
    .union(EventTypeFlags::INTERACTION_CREATE)
    .union(EventTypeFlags::DIRECT_MESSAGES)
    .union(EventTypeFlags::GUILD_CREATE);

#[tracing::instrument(skip_all, fields(
    ctx.latency = ?shard.recent_latency(),
    guild.id = ?event.guild_id(),
    event.kind = ?event.kind(),
    shard.id = %shard.id,
))]
pub async fn handle_event(shard: ShardContext, event: Event) {
    let event_kind = event.kind();
    let result: Result<()> = match event {
        Event::GuildCreate(data) => self::guild_create::handle(&shard, data.0).await,
        Event::InteractionCreate(data) => self::interaction::handle(&shard, data.0).await,
        Event::Ready(data) => self::ready::handle(&shard, &data).await,
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
