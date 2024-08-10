use crate::interactions::commands::{CommandContext, RunCommand};
use eden_discord_types::commands::local_guild::PayerCommand;
use twilight_model::guild::Permissions;

mod application;
mod pay_bill;
mod register;

impl RunCommand for PayerCommand {
    async fn run(&self, ctx: &CommandContext) -> eden_utils::Result<()> {
        match self {
            Self::Application(cmd) => cmd.run(ctx).await,
            Self::PayBill(cmd) => cmd.run(ctx).await,
            Self::Register(cmd) => cmd.run(ctx).await,
            Self::Test(..) => ctx.unimplemented_cmd(),
        }
    }

    fn guild_permissions(&self) -> Permissions {
        match self {
            Self::Application(cmd) => cmd.guild_permissions(),
            Self::PayBill(cmd) => cmd.guild_permissions(),
            Self::Register(cmd) => cmd.guild_permissions(),
            Self::Test(..) => Permissions::empty(),
        }
    }

    fn user_permissions(&self) -> Permissions {
        match self {
            Self::Application(cmd) => cmd.user_permissions(),
            Self::PayBill(cmd) => cmd.user_permissions(),
            Self::Register(cmd) => cmd.user_permissions(),
            Self::Test(..) => Permissions::empty(),
        }
    }

    fn channel_permissions(&self) -> Permissions {
        match self {
            Self::Application(cmd) => cmd.channel_permissions(),
            Self::PayBill(cmd) => cmd.channel_permissions(),
            Self::Register(cmd) => cmd.channel_permissions(),
            Self::Test(..) => Permissions::empty(),
        }
    }
}
