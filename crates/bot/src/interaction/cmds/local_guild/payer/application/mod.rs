use crate::interaction::cmds::{CommandContext, RunCommand};
use eden_bot_definitions::cmds::local_guild::PayerApplicationCommand;

mod status;

impl RunCommand for PayerApplicationCommand {
    async fn run(&self, ctx: &CommandContext<'_>) -> eden_utils::Result<()> {
        match self {
            Self::Status(cmd) => cmd.run(ctx).await,
        }
    }
}
