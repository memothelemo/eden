use eden_utils::Result;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_util::builder::InteractionResponseDataBuilder;

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
    #[tracing::instrument(skip(ctx))]
    async fn run(&self, ctx: &CommandContext<'_>) -> Result<()> {
        let data = InteractionResponseDataBuilder::new().content("Hi!").build();
        ctx.respond(data).await
    }
}
