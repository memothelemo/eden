use eden_utils::Result;
use tracing::warn;
use twilight_model::application::interaction::InteractionType;
use twilight_model::{
    application::interaction::InteractionData, gateway::payload::incoming::InteractionCreate,
};

use crate::interactions::CommandContext;
use crate::shard::ShardContext;

#[tracing::instrument(skip_all, fields(
    interaction.author = ?interaction.author_id(),
    interaction.channel.id = ?interaction.channel.as_ref().map(|v| v.id),
    interaction.kind = ?interaction.kind,
    interaction.is_guild = ?interaction.is_guild(),
    interaction.locale = ?interaction.locale,
))]
pub async fn handle(ctx: &ShardContext, interaction: InteractionCreate) -> Result<()> {
    let Some(data) = &interaction.data else {
        return Ok(());
    };

    // TODO: Handle application command errors
    match data {
        InteractionData::ApplicationCommand(data) => {
            let data = *data.clone();
            let ctx = CommandContext::new(ctx.bot.clone(), interaction.0, data, ctx);
            match ctx.interaction.kind {
                InteractionType::ApplicationCommand => {
                    crate::interactions::commands::handle(ctx).await?
                }
                _ => {
                    warn!("unimplemented {:?} interaction type", ctx.interaction.kind);
                }
            }
        }
        _ => {
            warn!("unimplemented {:?} interaction type", interaction.kind);
        }
    }

    Ok(())
}
