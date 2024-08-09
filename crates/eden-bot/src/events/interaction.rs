use eden_utils::Result;
use tracing::{debug, warn};
use twilight_model::application::interaction::{
    application_command::CommandData, Interaction, InteractionData, InteractionType,
};

use super::EventContext;
use crate::interactions::commands::CommandContext;

#[tracing::instrument(skip_all, fields(
    interaction.channel.id = ?interaction.channel.as_ref().map(|v| v.id),
    interaction.kind = ?interaction.kind,
    interaction.invoker = ?interaction.author_id(),
    interaction.is_guild = ?interaction.is_guild(),
    interaction.locale = ?interaction.locale,
))]
pub async fn handle(ctx: &EventContext, interaction: Interaction) -> Result<()> {
    let Some(data) = &interaction.data else {
        warn!("got interaction with no data");
        return Ok(());
    };

    let kind = interaction.kind;
    let result = match data {
        InteractionData::ApplicationCommand(data) => {
            let data = *data.clone();
            handle_command(ctx, data, interaction).await
        }
        _ => {
            warn!("got unimplemented {kind:?} interaction type");
            Ok(())
        }
    };

    if let Err(error) = result {
        warn!(%error, "could not process interaction {kind:?}");
    }

    Ok(())
}

#[tracing::instrument(skip_all, fields(
    command.id = ?data.id,
    command.name = tracing::field::Empty,
    command.kind = ?data.kind,
    command.guild_id = ?data.guild_id,
))]
async fn handle_command(
    ctx: &EventContext,
    data: CommandData,
    interaction: Interaction,
) -> Result<()> {
    debug!("received command interaction");

    let command_ctx = CommandContext::new(ctx.bot.clone(), ctx, data, &interaction);
    match command_ctx.interaction.kind {
        InteractionType::ApplicationCommand => {
            let span = tracing::Span::current();
            if !span.is_disabled() {
                span.record(
                    "command.name",
                    tracing::field::display(command_ctx.command_name()),
                );
            }
            // we cannot guarantee that commands do run fast
            command_ctx.defer(false).await?;
            crate::interactions::commands::handle(command_ctx).await?;
        }
        unknown => {
            warn!("got unimplemented {unknown:?} interaction type");
        }
    }
    Ok(())
}
