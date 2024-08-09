use eden_utils::Result;
use tracing::debug;
use twilight_model::channel::Message;

use super::EventContext;
use crate::interactions::StatefulCommandTrigger;

#[allow(clippy::expect_used)]
#[tracing::instrument(skip_all, fields(
    %message.id,
    %message.author.id,
    %message.channel_id,
    ?message.guild_id,
    ?message.kind,
    ?message.timestamp,
))]
pub async fn handle(ctx: &EventContext, message: Message) -> Result<()> {
    if message.author.bot {
        return Ok(());
    }

    debug!("received human message {}", message.id);
    ctx.bot
        .command_state
        .trigger_command(StatefulCommandTrigger::SentMessage(
            message.channel_id,
            message.id,
        ))
        .await;

    Ok(())
}
