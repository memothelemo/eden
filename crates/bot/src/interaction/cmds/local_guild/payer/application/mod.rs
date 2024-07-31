use crate::interaction::cmds::{CommandContext, RunCommand};
use eden_bot_definitions::cmds::local_guild::PayerApplicationCommand;

mod pending;
mod status;

impl RunCommand for PayerApplicationCommand {
    async fn run(&self, ctx: &CommandContext<'_>) -> eden_utils::Result<()> {
        match self {
            Self::Pending(cmd) => cmd.run(ctx).await,
            Self::Status(cmd) => cmd.run(ctx).await,
        }
    }
}
