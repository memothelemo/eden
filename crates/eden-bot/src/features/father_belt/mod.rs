use eden_utils::twilight::error::TwilightHttpErrorExt;
use tracing::{instrument, trace, warn};
use twilight_model::channel::Message;

use crate::events::EventContext;
use crate::util::http::request_for_model;

mod introduce;

#[instrument(skip_all)]
pub async fn on_message_create(ctx: &EventContext, message: &Message) {
    self::introduce::on_trigger(ctx, message).await;

    // TODO: check channel permissions first before sending the message
    if is_screaming(&message.content) && message.guild_id.is_some() {
        trace!("alerting the user not to scream");

        let request = ctx
            .bot
            .http
            .create_message(message.channel_id)
            .content("Keep your voice down!")
            .unwrap()
            .reply(message.id);

        if let Err(error) = request_for_model(&ctx.bot.http, request).await {
            let error = error.anonymize();
            let has_missing_access = error
                .discord_http_error_info()
                .map(|v| v.has_missing_access())
                .unwrap_or_default();

            if !has_missing_access {
                warn!(%error, "could not alert all caps message warning to the user");
            }
        }
    }
}

// - Messages with only non-alphabetic characters are not considered as screaming
// - Messages that can be considered as screaming if there are more than 2 consecutive
//   uppercased words
// - Messages with one word but more than 10 characters that are all uppercased
// - Aggressive amounts of exclamation marks (3 perhaps) are considered screaming
//
// List may go down but this will be our mechanism for now.
fn is_screaming(content: &str) -> bool {
    const AGGRESSIVE_MARKS: usize = 2;

    let words = content.split(" ").collect::<Vec<_>>();
    for list in words.windows(2) {
        assert_eq!(list.len(), 2);

        let a = list[0].chars().all(|v| v.is_uppercase());
        let b = list[1].chars().all(|v| v.is_uppercase());
        if a == true && b == true {
            return true;
        }
    }

    let reached_aggressive_threshold =
        content.chars().filter(|v| *v == '!').count() >= AGGRESSIVE_MARKS;

    let has_alphabetic_chars = content.chars().any(|v| v.is_alphabetic());
    let more_than_6_chars = content.len() >= 6;
    let is_in_all_uppercase = content
        .chars()
        .filter(|v| v.is_alphabetic())
        .all(|v| v.is_uppercase());

    (has_alphabetic_chars && is_in_all_uppercase && more_than_6_chars)
        || reached_aggressive_threshold
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_screaming() {
        assert!(!is_screaming("I'm a cool guy"));
        assert!(!is_screaming("Hey wassup man!?"));

        // This message is not that screaming like
        assert!(!is_screaming("GG"));
        assert!(!is_screaming("GG"));
        assert!(!is_screaming("what the HECK?"));

        assert!(is_screaming("WHAT THE?"));
        assert!(is_screaming("GG!!!!!!!!!!!!!!!!!!!"));
        assert!(is_screaming("WHAT!!"));
    }
}
