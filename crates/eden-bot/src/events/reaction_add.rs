use eden_utils::Result;
use tracing::debug;
use twilight_model::{channel::message::ReactionType, gateway::GatewayReaction};

use super::EventContext;
use crate::interactions::{
    state::StatefulCommandTrigger,
    util::local_guild::{LEFT_EMOJI, RIGHT_EMOJI},
};

#[tracing::instrument(skip_all, fields(
    %reaction.channel_id,
    ?reaction.emoji,
    ?reaction.guild_id,
    ?reaction.member,
    %reaction.message_id,
    reaction.reactor = %reaction.user_id,
))]
pub async fn handle(ctx: &EventContext, reaction: GatewayReaction) -> Result<()> {
    debug!("received message reaction");

    let ReactionType::Unicode { name: emoji } = reaction.emoji else {
        return Ok(());
    };

    let reactor = reaction.user_id;
    let message_id = reaction.message_id;

    let trigger = match emoji.as_str() {
        LEFT_EMOJI => StatefulCommandTrigger::ReactedLeftArrow(reactor, message_id),
        RIGHT_EMOJI => StatefulCommandTrigger::ReactedRightArrow(reactor, message_id),
        _ => return Ok(()),
    };
    ctx.bot.command_state.trigger_commands(trigger);

    Ok(())
}
