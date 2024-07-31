use twilight_interactions::command::{CommandModel, CreateCommand};

#[derive(Debug, CreateCommand, CommandModel)]
#[command(
    name = "ping",
    desc = "This command is generally used to check if the bot is online"
)]
pub struct Ping {
    /// Whether the bot should show their recent latency in milliseconds.
    ///
    /// The bot's recent latency is the time it takes for the bot
    /// to receive a message from Discord after sending a message.
    pub show_latency: Option<bool>,
}
