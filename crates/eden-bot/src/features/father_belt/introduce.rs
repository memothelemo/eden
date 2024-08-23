use tracing::{instrument, trace};
use twilight_model::channel::Message;

use crate::events::EventContext;

#[instrument(skip_all)]
pub async fn on_trigger(ctx: &EventContext, message: &Message) {
    if message.guild_id.is_none() {
        return;
    }

    trace!("relying back introduction message");
}

// Bisaya and Filipino languages are not supported because of complexity
// and Filipino do sometimes mix some words to make it understandable
fn get_name(message: &Message) -> Option<String> {
    todo!()
}
