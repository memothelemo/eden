mod context;
mod guild_create;
mod ready;

pub use self::context::*;

use eden_utils::Result;
use tracing::{debug, warn};
use twilight_gateway::Event;

#[tracing::instrument(skip_all, fields(
    ctx.latency = ?ctx.get_latency(),
    ctx.shard.id = %ctx.shard.id(),
    event.guild.id = ?event.guild_id(),
    event.kind = ?event.kind(),
))]
pub async fn handle_event(ctx: EventContext, event: Event) {
    let event_kind = event.kind();
    let result: Result<()> = match event {
        Event::GuildCreate(guild) => self::guild_create::handle(&ctx, guild.0).await,
        // Event::InteractionCreate(data) => self::interaction::handle(&ctx, data.0).await,
        Event::Ready(data) => self::ready::handle(&ctx, &data).await,
        Event::Resumed => {
            debug!("successfully resumed gateway session");
            Ok(())
        }
        Event::GatewayClose(..) => Ok(()),
        _ => {
            warn!("received unimplemented {event_kind:?} event");
            Ok(())
        }
    };

    if let Err(error) = result {
        warn!(%error, "unhandled error from event {event_kind:?}");
    }
}
