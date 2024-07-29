use std::fmt::Display;
use twilight_util::builder::embed::EmbedBuilder;

pub fn error(title: impl Display) -> EmbedBuilder {
    EmbedBuilder::new().title(format!("🔴  {title}"))
}

const INTERNAL_ERROR_DESC: &str = r"There's something wrong with the bot while we process your command.

Please contact @memothelemo for your assistance.";

pub fn internal_error() -> EmbedBuilder {
    error("Something went wrong!").description(INTERNAL_ERROR_DESC)
}
