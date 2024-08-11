use crate::interactions::commands::{CommandContext, RunCommand};
use eden_discord_types::commands::local_guild::SettingsCommand;
use eden_utils::Result;
use std::fmt::Debug;
use twilight_model::guild::Permissions;
use twilight_util::builder::InteractionResponseDataBuilder;

mod payer;
mod user;

impl RunCommand for SettingsCommand {
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        match self {
            Self::Payer(cmd) => cmd.run(ctx).await,
            Self::User(cmd) => cmd.run(ctx).await,
        }
    }

    fn guild_permissions(&self) -> Permissions {
        match self {
            Self::Payer(cmd) => cmd.guild_permissions(),
            Self::User(cmd) => cmd.guild_permissions(),
        }
    }

    fn user_permissions(&self) -> Permissions {
        match self {
            Self::Payer(cmd) => cmd.user_permissions(),
            Self::User(cmd) => cmd.user_permissions(),
        }
    }
}

pub async fn reply_with_changed_value(
    ctx: &CommandContext,
    name: &str,
    value: impl Debug,
) -> Result<()> {
    let data = InteractionResponseDataBuilder::new()
        .content(format!("**Changed \"{name}\" to**: `{value:?}`"))
        .build();

    ctx.respond(data).await?;
    Ok(())
}

pub async fn reply_with_output(ctx: &CommandContext, name: &str, value: impl Debug) -> Result<()> {
    let data = InteractionResponseDataBuilder::new()
        .content(format!("**{name}**: `{value:?}`"))
        .build();

    ctx.respond(data).await?;
    Ok(())
}
