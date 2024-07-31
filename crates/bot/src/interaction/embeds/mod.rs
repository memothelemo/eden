use std::fmt::Display;
use twilight_util::builder::embed::EmbedBuilder;

pub mod colors {
    /// Eden's signature color red.
    pub const RED: u32 = 0xE83A27;

    /// Eden's signature color green.
    pub const GREEN: u32 = 0x40D151;
}
pub mod error;

#[must_use]
pub fn with_emoji(emoji: char, title: impl Display) -> EmbedBuilder {
    EmbedBuilder::new().title(format!("{emoji}  {title}"))
}

#[must_use]
pub fn success(title: impl Display) -> EmbedBuilder {
    EmbedBuilder::new()
        .title(format!("✅  {title}"))
        .color(colors::GREEN)
}
