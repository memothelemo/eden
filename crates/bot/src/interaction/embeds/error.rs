use chrono::{DateTime, Utc};
use itertools::Itertools;
use std::fmt::Display;
use twilight_model::channel::message::Embed;
use twilight_model::http::interaction::InteractionResponseData;
use twilight_model::id::marker::UserMarker;
use twilight_model::{id::Id, util::Timestamp};
use twilight_util::builder::embed::EmbedBuilder;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::Bot;

// TODO: Implement id based interactions so I can diagnose errors faster
pub fn custom(title: impl Display, now: Option<DateTime<Utc>>) -> EmbedBuilder {
    let mut builder = EmbedBuilder::new()
        .title(format!("🔴  {title}"))
        .color(0xE83A27);

    // twilight uses 'time' while Eden uses 'chrono'
    if let Some(now) = now {
        match Timestamp::from_secs(now.timestamp()) {
            Ok(timestamp) => {
                builder = builder.timestamp(timestamp);
            }
            Err(error) => {
                tracing::error!(%error, "could not convert chrono timestamp time to twilight's timestamp");
            }
        }
    }

    builder
}

pub fn internal_error(
    ctx: &Bot,
    error: &eden_utils::Error,
    invoker_id: Option<Id<UserMarker>>,
) -> InteractionResponseData {
    const DESCRIPTION: &str = r"There's something wrong with while I am processing your command.

Please contact @memothelemo to be able assist the problem.";

    let mut embeds = Vec::new();
    let special_error = match invoker_id {
        Some(n) => ctx.settings.bot.is_developer_user(n),
        None => false,
    };

    if special_error {
        print_error(error, &mut embeds);
    } else {
        let embed = self::custom("Something went wrong!", None)
            .description(DESCRIPTION)
            .build();

        embeds.push(embed);
    }

    let mut data = InteractionResponseDataBuilder::new().embeds(embeds);
    if special_error {
        data = data.content("🔴  **Error occurred!**");
    }

    data.build()
}

fn print_error(error: &eden_utils::Error, embeds: &mut Vec<Embed>) {
    // Output includes some of ANSI escape sequences since tracing_error
    // renders out the span trace by using the global subscriber set
    // from tracing crate.
    let output = strip_ansi_escapes::strip_str(error.to_string());

    // Split into chunks where each of them has a size is 4000
    // characters long only (96 characters away from Discord's maximum
    // amount of characters for embed descriptions)
    let chunks = output.chars().chunks(4000);
    let chunks = chunks.into_iter().map(|v| v.collect::<String>());

    for chunk in chunks {
        // Maximum amount of embeds for interaction response
        if embeds.len() == 10 {
            break;
        }

        let embed = EmbedBuilder::new()
            .description(format!("```{chunk}```"))
            .build();

        embeds.push(embed);
    }
}
