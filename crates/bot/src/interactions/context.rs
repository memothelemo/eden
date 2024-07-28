use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::AllowedMentions;
use twilight_util::builder::InteractionResponseDataBuilder;

use crate::Bot;

pub type CommandContext = InteractionContext<CommandData>;

#[derive(Debug, Clone)]
pub struct InteractionContext<T> {
    pub bot: Bot,
    pub interaction: Interaction,
    pub data: T,
}

impl<T> InteractionContext<T> {
    pub fn response(&self) -> InteractionResponseDataBuilder {
        InteractionResponseDataBuilder::new().allowed_mentions(AllowedMentions::default())
    }
}
