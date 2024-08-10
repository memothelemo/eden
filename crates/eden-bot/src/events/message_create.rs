use eden_utils::Result;
use tracing::trace;
use twilight_model::channel::Message;

use super::EventContext;
use crate::interactions::state::StatefulCommandTrigger;

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

    trace!("received human message {}", message.id);

    let trigger = StatefulCommandTrigger::SentMessage(message.channel_id, message.id);
    ctx.bot.command_state.trigger_commands(trigger);

    Ok(())
}
