use crate::interactions::commands::{CommandContext, RunCommand};
use eden_discord_types::commands::local_guild::PayerApplicationCommand;
use twilight_model::guild::Permissions;

mod pending;
mod status;

impl RunCommand for PayerApplicationCommand {
    async fn run(&self, ctx: &CommandContext) -> eden_utils::Result<()> {
        match self {
            Self::Pending(cmd) => cmd.run(ctx).await,
            Self::Status(cmd) => cmd.run(ctx).await,
        }
    }

    fn guild_permissions(&self) -> Permissions {
        match self {
            Self::Pending(cmd) => cmd.guild_permissions(),
            Self::Status(cmd) => cmd.guild_permissions(),
        }
    }

    fn user_permissions(&self) -> Permissions {
        match self {
            Self::Pending(cmd) => cmd.user_permissions(),
            Self::Status(cmd) => cmd.user_permissions(),
        }
    }

    fn channel_permissions(&self) -> Permissions {
        match self {
            Self::Pending(cmd) => cmd.channel_permissions(),
            Self::Status(cmd) => cmd.channel_permissions(),
        }
    }
}
