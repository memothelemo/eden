use eden_discord_types::commands::local_guild::{UserSettingsCommand, UserSettingsDeveloperMode};
use eden_schema::{forms::UpdateUserForm, types::User};
use eden_utils::{error::exts::*, Result};
use tracing::trace;

use super::{CommandContext, RunCommand};

impl RunCommand for UserSettingsCommand {
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        match self {
            UserSettingsCommand::DeveloperMode(cmd) => cmd.run(ctx).await,
        }
    }
}

impl RunCommand for UserSettingsDeveloperMode {
    #[tracing::instrument(skip(ctx))]
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        // try to load user's settings if possible
        let mut conn = ctx.bot.db_write().await?;
        let invoker_id = ctx.invoker_id();
        let user = User::get_or_insert(&mut conn, invoker_id).await?;

        if let Some(overwrite) = self.set {
            trace!("overriding 'developer_mode' for user {invoker_id}");

            let form = UpdateUserForm::builder()
                .developer_mode(Some(overwrite))
                .build();

            User::update(&mut conn, invoker_id, form).await?;
            conn.commit()
                .await
                .into_eden_error()
                .attach_printable("could not commit transaction")?;

            super::reply_with_changed_value(ctx, "Developer Mode", overwrite).await
        } else {
            trace!("getting 'developer_mode' for user {invoker_id}");
            super::reply_with_output(ctx, "Developer Mode", user.developer_mode).await
        }
    }
}
