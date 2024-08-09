use eden_discord_types::commands::local_guild::{
    PayerSettingsAllowSelfRegistration, PayerSettingsCommand,
};
use eden_schema::types::GuildSettings;
use eden_utils::{error::exts::*, Result};
use tracing::trace;
use twilight_model::guild::Permissions;

use super::{CommandContext, RunCommand};
use crate::interactions::{record_local_guild_ctx, LocalGuildContext};

impl RunCommand for PayerSettingsCommand {
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        match self {
            Self::AllowSelfRegistration(cmd) => cmd.run(ctx).await,
        }
    }

    fn user_permissions(&self) -> Permissions {
        match self {
            Self::AllowSelfRegistration(cmd) => cmd.user_permissions(),
        }
    }

    fn guild_permissions(&self) -> Permissions {
        match self {
            Self::AllowSelfRegistration(cmd) => cmd.guild_permissions(),
        }
    }
}

impl RunCommand for PayerSettingsAllowSelfRegistration {
    #[tracing::instrument(skip(ctx), fields(ctx = tracing::field::Empty))]
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        let ctx = LocalGuildContext::from_ctx(ctx).await?;
        record_local_guild_ctx!(ctx);

        if let Some(overwrite) = self.set {
            trace!("overriding `allow_self_registration` to {overwrite}");

            let mut conn = ctx.bot.db_write().await?;
            let mut form = ctx.settings.data.clone();
            form.payers.allow_self_register = overwrite;

            GuildSettings::update(&mut conn, ctx.guild_id, &form).await?;
            conn.commit()
                .await
                .into_eden_error()
                .attach_printable("could not commit transaction")?;

            super::reply_with_changed_value(&ctx, "Allow self registration", overwrite).await
        } else {
            trace!("getting `allow_self_registration` value");
            super::reply_with_output(
                ctx.inner,
                "Allow self registration",
                ctx.settings.payers.allow_self_register,
            )
            .await
        }
    }

    fn user_permissions(&self) -> Permissions {
        Permissions::ADMINISTRATOR
    }
}
