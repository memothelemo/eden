use eden_utils::{Error, Result};
use thiserror::Error;
use twilight_model::gateway::payload::incoming::Ready;

use crate::shard::ShardContext;

#[derive(Debug, Error)]
#[error("unable to override application id")]
struct SetApplicationIdError;

pub fn handle(ctx: &ShardContext, data: &Ready) -> Result<()> {
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
        let id_ref = &ctx.bot.application_id;
        id_ref
            .set(actual_id)
            .map_err(|_| Error::unknown(SetApplicationIdError))?;
    }

    Ok(())
}
