use eden_tasks::Scheduled;
use eden_utils::Result;
use tracing::{debug, warn};
use twilight_model::guild::Guild;

use crate::tasks;

use super::EventContext;

#[allow(clippy::expect_used)]
#[tracing::instrument(skip_all, fields(
    %guild.id,
    guild.members = %guild.member_count.unwrap_or_default(),
))]
pub async fn handle(ctx: &EventContext, guild: Guild) -> Result<()> {
    if !ctx.bot.is_local_guild(&guild) {
        return Ok(());
    }

    // We may want to load their settings in and save it as cache
    ctx.bot.on_local_guild_loaded();
    debug!("found local guild of {}", guild.id);

    if let Err(error) = crate::local_guild::setup(&ctx.bot, &guild).await {
        let error = error.anonymize();
        warn!(%error, "unable to setup local guild. scheduling task to setup local guild later...");

        let task = tasks::SetupLocalGuild;
        ctx.bot
            .queue
            .schedule(task, Scheduled::in_minutes(2))
            .await?;
    }

    Ok(())
}
