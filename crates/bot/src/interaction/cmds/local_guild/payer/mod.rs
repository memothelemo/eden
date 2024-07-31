use crate::interaction::cmds::RunCommand;

use eden_bot_definitions::cmds::local_guild::PayerCommand;

mod register;

impl RunCommand for PayerCommand {
    async fn run(
        &self,
        ctx: &crate::interaction::cmds::CommandContext<'_>,
    ) -> eden_utils::Result<()> {
        match self {
            PayerCommand::Register(cmd) => cmd.run(ctx).await,
            PayerCommand::Test(..) => {
                ctx.defer(true).await?;
                Ok(())
            }
        }
    }
}
