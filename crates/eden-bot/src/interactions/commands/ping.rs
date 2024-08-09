use eden_discord_types::commands::Ping;
use eden_utils::{error::exts::*, Result};
use fancy_duration::FancyDuration;
use std::fmt::Write as _;
use std::time::Duration;
use tracing::trace;
use twilight_model::channel::message::Embed;
use twilight_util::builder::InteractionResponseDataBuilder;

use super::{CommandContext, RunCommand};
use crate::interactions::embeds;

impl RunCommand for Ping {
    #[tracing::instrument(skip(ctx))]
    async fn run(&self, ctx: &CommandContext) -> Result<()> {
        let mut content = "**:ping_pong:  Pong!**".to_string();
        let mut data = InteractionResponseDataBuilder::new();

        let show_latency = self.show_latency.unwrap_or_default();
        if show_latency {
            let latency = get_latency(ctx).await;
            trace!(?latency);

            if let Some(latency) = latency {
                write!(&mut content, " (*{latency}*)")
                    .into_typed_error()
                    .anonymize_error()
                    .attach_printable("could not append string to display latency")?;
            } else {
                let embed = not_latency_error_embed();
                data = data.embeds(vec![embed]);
            }
        }

        let data = data.content(content).build();
        ctx.respond(data).await
    }
}

async fn get_latency(ctx: &CommandContext) -> Option<FancyDuration<Duration>> {
    let latency = ctx.shard.latency().await;
    let recent = latency.recent().first();
    recent.map(|v| FancyDuration(*v).truncate(1))
}

// most likely the cause of this error because the invoker uses the
// ping command with show_latency on too early after the bot has
// been started.
fn not_latency_error_embed() -> Embed {
    const MESSAGE: &str = "I'm waiting to get my latency at the moment.\nPlease try again in a short while (around a minute will do so).";
    embeds::builders::error("Unable to show latency!", None)
        .description(MESSAGE)
        .build()
}
