use crate::interaction::commands::{Command, CommandContext};
use twilight_interactions::command::{CommandModel, CreateCommand};

mod register;
mod test;

pub use self::register::*;
pub use self::test::*;

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "payer",
    desc = "Commands to manage things as a payer",
    dm_permission = false
)]
pub enum PayerCommand {
    #[command(name = "register")]
    Register(PayerRegister),
    #[command(name = "test")]
    Test(PayerTest),
}

impl Command for PayerCommand {
    async fn run(&self, ctx: &CommandContext<'_>) -> eden_utils::Result<()> {
        match self {
            Self::Register(command) => command.run(ctx).await,
            Self::Test(command) => command.run(ctx).await,
        }
    }
}
