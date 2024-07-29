use crate::shard::ShardContext;
use eden_utils::Result;
use twilight_model::application::interaction::Interaction;

#[tracing::instrument(skip_all, fields(
    interaction.author = ?interaction.author_id(),
    interaction.channel.id = ?interaction.channel.as_ref().map(|v| v.id),
    interaction.kind = ?interaction.kind,
    interaction.is_guild = ?interaction.is_guild(),
    interaction.locale = ?interaction.locale,
))]
pub async fn handle(_ctx: &ShardContext, interaction: Interaction) -> Result<()> {
    todo!()
}
