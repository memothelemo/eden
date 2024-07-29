use eden_utils::Result;
use twilight_model::gateway::payload::incoming::Ready;

use crate::shard::ShardContext;

#[tracing::instrument(skip_all, fields(
    %data.application.id,
    data.guilds.len = %data.guilds.len(),
    %data.version,
))]
pub async fn handle(ctx: &ShardContext, data: &Ready) -> Result<()> {
    tracing::debug!("shard is ready");

    let actual_id = data.application.id;
    let configured_id = ctx.bot.settings.bot.application_id;
    let expected_id = ctx.bot.application_id.get().cloned().or(configured_id);

    let same_as_configured = expected_id.map(|v| v == actual_id).unwrap_or(true);
    if !same_as_configured {
        tracing::warn!(
            ?actual_id,
            ?expected_id,
            "unmatched application IDs. replacing with actual application ID"
        );

        // take it first heheheh
        if ctx.bot.application_id.set(actual_id).is_err() {
            tracing::warn!("could not replace new application ID");
        }
    }

    Ok(())
}
