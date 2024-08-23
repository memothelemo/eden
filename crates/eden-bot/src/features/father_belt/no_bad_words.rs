use difference::{Changeset, Difference};
use eden_utils::twilight::error::TwilightHttpErrorExt;
use itertools::Itertools;
use rand::Rng;
use rustrict::{Trie, Type};
use std::sync::LazyLock;
use tokio::task::spawn_blocking;
use tracing::{instrument, trace, warn};
use twilight_model::channel::Message;

use crate::events::EventContext;
use crate::util::http::request_for_model;

#[instrument(skip_all)]
pub async fn on_trigger(ctx: &EventContext, message: &Message) -> bool {
    if message.guild_id.is_none() {
        return false;
    }

    // We only limit up to 1500 characters unfortunately :)
    let limit = message.content.len().clamp(1, 1500);
    let original = message.content[..limit].to_string();

    // read the comment from process_bad_words function to see why
    // we need to use spawn_blocking for this kind of task
    //
    // also, ThreadRng is not safe to use in this context so we need
    // to include it as well here.
    let result = spawn_blocking(move || {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..WARN_MESSAGES.len());
        let warn_message = WARN_MESSAGES[index];
        (process_bad_words(&original), warn_message)
    })
    .await;

    let Ok((bad_words, warn_message)) = result else {
        return false;
    };

    // we don't need to warn the user if they swore something
    if bad_words.is_empty() {
        return false;
    }

    // render it letter by letter
    //
    // For example:
    // `foo` -> `f o o`
    let bad_words = bad_words
        .into_iter()
        .map(|v| format!("`{}`", v.chars().join(" ")))
        .join(", ");

    let preferred_name = message
        .member
        .as_ref()
        .and_then(|v| v.nick.as_deref())
        .unwrap_or_else(|| message.author.name.as_str());

    let content = warn_message
        .replace("{USER_NAME}", preferred_name)
        .replace("{BAD_WORDS}", &bad_words);

    let request = ctx
        .bot
        .http
        .create_message(message.channel_id)
        .content(&content)
        .unwrap()
        .reply(message.id);

    trace!("warning the user to not swear");
    if let Err(error) = request_for_model(&ctx.bot.http, request).await {
        let has_missing_access = error
            .discord_http_error_info()
            .map(|v| v.has_missing_access())
            .unwrap_or_default();

        if !has_missing_access {
            warn!(%error, "could not warn the user with message to not swear");
        }
    }

    true
}

const WARN_MESSAGES: &[&str] = &[
    // copied from dad bot. sorry!
    "Listen here {USER_NAME}, I will not tolerate you saying the words that consist of the letters {BAD_WORDS} being said in this server, so take your own advice and close thine mouth in the name of the christian minecraft server owner.",
    "Did your mom told you not to say {BAD_WORDS} to everyone? If you have nothing nice to say in this server, then shut up!",
    "You said {BAD_WORDS}. My goodness, you're a bad person!",
    "Did you know that {BAD_WORDS} is/are bad words?",
    "> *Do not let any unwholesome talk come out of your mouths, but only what is helpful for building others up according to their needs, that it may benefit those who listen.*\n> \n> Ephesians 4:29 (NIV)",
];

static FILIPINO_BAD_WORDS: LazyLock<Trie> = LazyLock::new(|| {
    let mut trie = Trie::new();
    trie.set("gago", Type::PROFANE);
    trie.set("gaga", Type::PROFANE);
    trie.set("yawa", Type::PROFANE);
    trie.set("puta", Type::PROFANE);
    trie.set("putang", Type::PROFANE);
    trie.set("putang", Type::PROFANE);
    trie.set("tangina", Type::PROFANE);
    trie.set("bobo", Type::PROFANE);
    trie.set("syet", Type::PROFANE);
    trie.set("buwisit", Type::PROFANE);
    trie.set("bwisit", Type::PROFANE);
    trie.set("amputa", Type::PROFANE);
    trie.set("bilat", Type::PROFANE);
    trie.set("gagi", Type::PROFANE);
    trie.set("iyot", Type::PROFANE);
    trie.set("leche", Type::PROFANE);
    trie.set("lintik", Type::PROFANE);
    trie.set("shet", Type::PROFANE);
    trie.set("puke", Type::PROFANE);
    trie.set("suso", Type::PROFANE);
    trie.set("tae", Type::PROFANE);
    trie.set("taena", Type::PROFANE);
    trie.set("tete", Type::PROFANE);
    trie.set("tite", Type::PROFANE);
    trie.set("titi", Type::PROFANE);
    trie.set("ungas", Type::PROFANE);
    trie.set("tanga", Type::PROFANE);
    trie
});

fn process_bad_words(content: &str) -> Vec<String> {
    let mut bad_words = Vec::new();

    // this is to avoid like in issue #9 but it will process words SLOWER
    for original in content.split_whitespace() {
        if url::Url::parse(original).is_ok() {
            continue;
        }

        // this will make my life easier when diff'ing strings later on
        let censored = super::init_censor!(original)
            .with_censor_first_character_threshold(*super::RUSTRICT_CONFIGURED_TYPE)
            .with_trie(&FILIPINO_BAD_WORDS)
            .censor();

        let changeset = Changeset::new(original, &censored, "");
        for diff in changeset.diffs {
            if let Difference::Rem(original) = diff {
                bad_words.push(original.to_lowercase());
            }
        }
    }

    bad_words
}

#[cfg(test)]
mod tests {
    use super::*;

    // This is just for testing purposes only and it is not
    // intended to hurt anyone. :)
    //
    // Sorry if your feelings got hurt because of these sentences.
    #[test]
    fn test_process_bad_words() {
        assert_eq!(process_bad_words("How fucking dare you!"), &["fucking"]);
        assert_eq!(process_bad_words("Shit bitch"), &["shit", "bitch"]);
        assert_eq!(process_bad_words("shit bitch"), &["shit", "bitch"]);
        assert!(process_bad_words("No bad words here!").is_empty());
    }
}
