use eden_utils::Result;
use tracing::debug;
use twilight_model::gateway::payload::incoming::Ready;

use super::EventContext;

#[tracing::instrument(skip_all, fields(
    %data.application.id,
    data.guilds.len = %data.guilds.len(),
    %data.version,
))]
pub async fn handle(_ctx: &EventContext, data: &Ready) -> Result<()> {
    // application id is overriden from ShardRunner
    debug!(
        "logged in as {:?} ({})",
        data.user.name, data.application.id
    );
    Ok(())
}
