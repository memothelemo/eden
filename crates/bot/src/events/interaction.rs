use crate::interaction::{self, commands::CommandContext};
use crate::shard::ShardContext;

use eden_utils::Result;
use tracing::warn;
use twilight_model::application::interaction::{
    application_command::CommandData, Interaction, InteractionData, InteractionType,
};

#[tracing::instrument(skip_all, fields(
    interaction.channel.id = ?interaction.channel.as_ref().map(|v| v.id),
    interaction.kind = ?interaction.kind,
    interaction.invoker = ?interaction.author_id(),
    interaction.is_guild = ?interaction.is_guild(),
    interaction.locale = ?interaction.locale,
))]
pub async fn handle(ctx: &ShardContext, interaction: Interaction) -> Result<()> {
    let Some(data) = &interaction.data else {
        tracing::warn!("got interaction with no data");
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
    command.name = %data.name,
    command.kind = ?data.kind,
    command.guild_id = ?data.guild_id,
))]
async fn handle_command(
    ctx: &ShardContext,
    data: CommandData,
    interaction: Interaction,
) -> Result<()> {
    let ctx = CommandContext::new(ctx.bot.clone(), &interaction, data, ctx);
    match ctx.interaction.kind {
        InteractionType::ApplicationCommand => {
            // update it into the actual one
            let span = tracing::Span::current();
            span.record("command.name", tracing::field::display(ctx.command_name()));
            interaction::commands::handle(ctx).await?;
        }
        unknown => {
            warn!("got unimplemented {unknown:?} interaction type");
        }
    };
    Ok(())
}
