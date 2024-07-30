use chrono::{DateTime, Utc};
use itertools::Itertools;
use std::fmt::Display;
use twilight_model::{
    http::interaction::InteractionResponseData,
    id::{marker::UserMarker, Id},
    util::Timestamp,
};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

use crate::Bot;

pub fn access_denied() -> EmbedBuilder {
    EmbedBuilder::new()
        .title(format!("❌  Access denied"))
        .color(0xE83A27)
}

// TODO: Implement id based interactions so I can diagnose errors faster
pub fn error(title: impl Display, now: Option<DateTime<Utc>>) -> EmbedBuilder {
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

const INTERNAL_ERROR_DESC: &str = r"There's something wrong with the bot while we process your command.

Please contact @memothelemo to be able assist the problem.";

pub fn internal_error(
    ctx: &Bot,
    user_id: Option<Id<UserMarker>>,
    error: &eden_utils::Error,
    now: DateTime<Utc>,
) -> InteractionResponseData {
    let mut embeds = Vec::new();
    let mut is_special = false;

    if let Some(user_id) = user_id
        && ctx.settings.bot.is_developer_user(user_id)
    {
        is_special = true;

        // Print the error and split each part per 4000
        // characters (96 characters away from max on Discord)
        let chunks = error
            .to_string()
            .chars()
            .chunks(4000)
            .into_iter()
            .map(|v| v.collect::<String>())
            .collect::<Vec<_>>();

        for chunk in chunks {
            if embeds.len() == 10 {
                break;
            }

            let embed = EmbedBuilder::new()
                .description(format!("```{chunk}```"))
                .build();

            embeds.push(embed);
        }
    } else {
        let embed = self::error("Something went wrong!", Some(now))
            .description(INTERNAL_ERROR_DESC)
            .build();

        embeds.push(embed);
    }

    let mut data = InteractionResponseDataBuilder::new().embeds(embeds);
    if is_special {
        data = data.content("**Error occurred!**");
    }

    data.build()
}
