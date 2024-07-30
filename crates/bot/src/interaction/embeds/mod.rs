use chrono::{DateTime, Utc};
use std::fmt::Display;
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::EmbedBuilder;

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

pub fn internal_error(now: DateTime<Utc>) -> EmbedBuilder {
    error("Something went wrong!", Some(now)).description(INTERNAL_ERROR_DESC)
}
