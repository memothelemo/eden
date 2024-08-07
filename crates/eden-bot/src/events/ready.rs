use eden_utils::Result;
use tracing::debug;
use twilight_model::gateway::payload::incoming::Ready;

use super::EventContext;

#[tracing::instrument(skip_all, fields(
    %data.application.id,
    data.guilds.len = %data.guilds.len(),
    %data.version,
))]
pub async fn handle(ctx: &EventContext, data: &Ready) -> Result<()> {
    debug!(
        "logged in as {:?} ({})",
        data.user.name, data.application.id
    );
    if ctx.bot.application_id().is_none() {
        ctx.bot.override_application_id(data.application.id);
    }
    Ok(())
}
