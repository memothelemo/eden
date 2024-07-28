use async_trait::async_trait;
use eden_utils::Result;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::interactions::{commands::Command, CommandContext};

#[derive(CreateCommand, CommandModel)]
#[command(name = "ping", desc = "Replies back with `pong!`")]
pub struct Ping;

#[async_trait]
impl Command for Ping {
    async fn run_command(&self, ctx: CommandContext) -> Result<()> {
        Ok(())
    }
}
