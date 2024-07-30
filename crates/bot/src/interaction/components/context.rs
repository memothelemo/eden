use twilight_model::application::interaction::message_component::MessageComponentInteractionData;

use crate::interaction::InteractionContext;

pub type ComponentContext<'a> = InteractionContext<'a, MessageComponentInteractionData>;
impl<'a> ComponentContext<'a> {}
