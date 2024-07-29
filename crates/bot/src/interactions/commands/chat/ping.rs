use async_trait::async_trait;
use eden_utils::{
    error::{AnyResultExt, ResultExt},
    Result,
};
use fancy_duration::FancyDuration;
use std::fmt::Write as _;
use twilight_interactions::command::{CommandModel, CreateCommand};
use twilight_util::builder::{embed::EmbedFooterBuilder, InteractionResponseDataBuilder};

use crate::interactions::{commands::Command, CommandContext};

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "ping",
    desc = "This command is generally used to check if the bot is online"
)]
#[command(autocomplete = true)]
pub struct Ping {
    /// Whether the bot should show their recent latency in milliseconds.
    ///
    /// The bot's recent latency is the time it takes for the bot
    /// to receive a message from Discord after sending a message.
    show_latency: Option<bool>,
}

#[async_trait]
impl Command for Ping {
    async fn run_command(&self, ctx: CommandContext<'_>) -> Result<()> {
        let mut content = "**:ping_pong:  Pong!**".to_string();
        let mut data = InteractionResponseDataBuilder::new();

        let show_bot_latency = self.show_latency.unwrap_or_default();
        if show_bot_latency {
            let recent = ctx.shard.latency.recent().first();
            if let Some(recent) = recent {
                let recent = FancyDuration(*recent).truncate(1);
                write!(&mut content, " (*{recent}*)")
                    .anonymize_error()
                    .attach_printable("could not add more content to display latency")?;
            } else {
                let embed = crate::interactions::embeds::error("Unable to show latency!")
                    .description("I cannot give you my recent latency at the moment.")
                    .footer(
                        EmbedFooterBuilder::new(
                            "Please try again in a later time to get my recent latency.",
                        )
                        .build(),
                    )
                    .build();

                data = data.embeds(vec![embed]);
            }
        }

        let data = data.content(content).build();
        ctx.respond(data).await
    }
}
