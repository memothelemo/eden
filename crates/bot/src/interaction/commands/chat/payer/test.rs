use eden_utils::error::ResultExt;
use eden_utils::Result;
use twilight_interactions::command::{CommandModel, CreateCommand};

use crate::interaction::commands::choices::PaymentMethodOption;
use crate::interaction::commands::{Command, CommandContext};

#[derive(Debug, CreateCommand, CommandModel)]
#[command(name = "test", desc = "Just a testing command", dm_permission = false)]
pub struct PayerTest {
    /// Your preferred payment method
    #[allow(unused)]
    method: PaymentMethodOption,
}

impl Command for PayerTest {
    #[tracing::instrument(skip(_ctx))]
    async fn run(&self, _ctx: &CommandContext<'_>) -> Result<()> {
        tokio::fs::read("woopp!!").await.anonymize_error()?;
        Ok(())
    }
}
