use crate::interaction::cmds::{CommandContext, RunCommand};
use eden_bot_definitions::cmds::local_guild::PayerCommand;

mod application;
mod register;

impl RunCommand for PayerCommand {
    async fn run(&self, ctx: &CommandContext<'_>) -> eden_utils::Result<()> {
        match self {
            PayerCommand::Application(cmd) => cmd.run(ctx).await,
            PayerCommand::Register(cmd) => cmd.run(ctx).await,
            PayerCommand::Test(..) => {
                ctx.defer(true).await?;
                Ok(())
            }
        }
    }
}
