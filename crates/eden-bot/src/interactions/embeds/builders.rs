use chrono::{DateTime, Utc};
use std::fmt::Display;
use tracing::warn;
use twilight_model::util::Timestamp;
use twilight_util::builder::embed::EmbedBuilder;

#[must_use]
pub fn with_emoji(emoji: char, title: impl Display) -> EmbedBuilder {
    EmbedBuilder::new().title(format!("{emoji}  {title}"))
}

#[must_use]
pub fn error(title: impl Display, emitted_at: Option<DateTime<Utc>>) -> EmbedBuilder {
    let mut builder = EmbedBuilder::new()
        .title(format!("❌  {title}"))
        .color(super::colors::RED);

    // twilight uses 'time' while Eden uses 'chrono'
    if let Some(emitted_at) = emitted_at {
        match Timestamp::from_secs(emitted_at.timestamp()) {
            Ok(timestamp) => {
                builder = builder.timestamp(timestamp);
            }
            Err(error) => {
                warn!(%error, "could not convert chrono timestamp time to twilight's timestamp");
            }
        }
    }

    builder
}

#[must_use]
pub fn success(title: impl Display) -> EmbedBuilder {
    EmbedBuilder::new()
        .title(format!("✅  {title}"))
        .color(super::colors::GREEN)
}
