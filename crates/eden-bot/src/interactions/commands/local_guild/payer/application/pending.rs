use eden_discord_types::commands::local_guild::PayerApplicationPending;
use eden_schema::types::PayerApplication;
use eden_utils::Result;
use tracing::trace;
use twilight_model::guild::Permissions;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::interactions::{
    commands::{CommandContext, RunCommand},
    record_local_guild_ctx,
    state::{commands::PayerApplicationPendingState, StatefulCommand},
    util::local_guild::{react_lr_emojis, render_payer_application_embed},
    LocalGuildContext,
};

impl RunCommand for PayerApplicationPending {
    #[tracing::instrument(skip_all, fields(ctx = tracing::field::Empty))]
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        let ctx = LocalGuildContext::from_ctx(ctx).await?;
        record_local_guild_ctx!(ctx);

        trace!("fetching payer application");
        let mut conn = ctx.bot.db_read().await?;
        let pending = PayerApplication::first_pending(&mut conn).await?;
        drop(conn);

        let mut embeds = Vec::new();
        let Some(application) = pending else {
            let data = InteractionResponseDataBuilder::new()
                .content("**There are no pending applications today! 🎉**");
            ctx.respond(data.build()).await?;
            return Ok(());
        };
        embeds.push(render_payer_application_embed(&application));

        let data = InteractionResponseDataBuilder::new().embeds(embeds);
        let message_id = ctx.respond(data.build()).await?;

        let state = PayerApplicationPendingState::new(
            ctx.channel_id,
            application.id,
            &ctx.interaction.token,
            ctx.author.id,
            message_id,
        );
        ctx.bot.command_state.insert(
            ctx.interaction.id,
            StatefulCommand::PayerApplicationPending(state),
        );
        react_lr_emojis(&ctx.bot, ctx.channel_id, message_id, false, true).await?;

        Ok(())
    }

    fn user_permissions(&self) -> Permissions {
        Permissions::ADMINISTRATOR
    }

    fn channel_permissions(&self) -> Permissions {
        Permissions::ADD_REACTIONS | Permissions::MANAGE_MESSAGES | Permissions::VIEW_CHANNEL
    }
}
