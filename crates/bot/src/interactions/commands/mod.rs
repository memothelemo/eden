use async_trait::async_trait;
use eden_utils::Result;
use twilight_interactions::command::{CommandModel, CreateCommand};

pub mod chat;

use super::CommandContext;

#[async_trait]
pub trait Command: CreateCommand + CommandModel {
    async fn run_command(&self, ctx: CommandContext) -> Result<()>;
}
