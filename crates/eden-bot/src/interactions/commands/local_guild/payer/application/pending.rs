use eden_discord_types::commands::local_guild::PayerApplicationPending;
use eden_utils::Result;
use twilight_model::guild::Permissions;

use crate::interactions::{
    commands::{CommandContext, RunCommand},
    record_local_guild_ctx, LocalGuildContext,
};

impl RunCommand for PayerApplicationPending {
    #[tracing::instrument(skip_all, fields(ctx = tracing::field::Empty))]
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        let ctx = LocalGuildContext::from_ctx(ctx).await?;
        record_local_guild_ctx!(ctx);

        ctx.unimplemented_cmd()
    }

    fn user_permissions(&self) -> Permissions {
        Permissions::ADMINISTRATOR
    }

    fn channel_permissions(&self) -> Permissions {
        Permissions::ADD_REACTIONS | Permissions::MANAGE_MESSAGES | Permissions::VIEW_CHANNEL
    }
}
