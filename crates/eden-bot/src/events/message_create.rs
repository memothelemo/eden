use eden_utils::{twilight::error::TwilightHttpErrorExt, Result};
use regex::Regex;
use rustrict::Censor;
use std::sync::LazyLock;
use tracing::{trace, warn};
use twilight_mention::Mention;
use twilight_model::{channel::Message, id::marker::UserMarker};

use super::EventContext;
use crate::{interactions::state::StatefulCommandTrigger, util::http::request_for_model};

#[allow(clippy::expect_used)]
#[tracing::instrument(skip_all, fields(
    %message.id,
    %message.author.id,
    %message.channel_id,
    ?message.guild_id,
    ?message.kind,
    ?message.timestamp,
))]
pub async fn handle(ctx: &EventContext, message: Message) -> Result<()> {
    if message.author.bot {
        return Ok(());
    }

    trace!("received human message {}", message.id);
    ctx.bot
        .command_state
        .trigger_commands(StatefulCommandTrigger::SentMessage(
            message.author.id,
            message.channel_id,
            message.id,
        ));

    // TODO: check channel permissions first before sending the message

    // don't actually do this if we're in dms
    if message.guild_id.is_some()
        && let Some(name) = process_name(&message.content)
    {
        trace!("relying back introductory message");

        if let Err(error) = respond_introduce_message(ctx, &message, &name).await {
            // if it is a missing access error, ignore it!
            let has_missing_access = error
                .discord_http_error_info()
                .map(|v| v.has_missing_access())
                .unwrap_or_default();

            if has_missing_access {
                return Ok(());
            }
            warn!(%error, "could not respond back introduction message to the user");
        }
    }

    Ok(())
}

// We don't want to let Eden say "Hi <swear word>" when the user said that so.
//
// By the way, this is inspired by Dad Bot#2189 made by alekeagle
#[tracing::instrument(skip_all)]
async fn respond_introduce_message(
    ctx: &EventContext,
    message: &Message,
    name: &str,
) -> Result<()> {
    // We only limit up to 1500 characters unfortunately :)
    let original_size = name.len();
    let limit = original_size.clamp(1, 1500);

    // censor some profanity HAHAHAH
    let mut name = Censor::from_str(&name[0..limit])
        .with_censor_threshold(
            rustrict::Type::INAPPROPRIATE
                | rustrict::Type::EVASIVE
                | rustrict::Type::OFFENSIVE
                | rustrict::Type::SEVERE,
        )
        .with_ignore_self_censoring(true)
        .with_censor_replacement('x')
        .censor();

    if name.len() != limit {
        name.push_str("...");
    }

    let content = format!(
        "Hi **{name}**, I'm {}!",
        ctx.bot.application_id().cast::<UserMarker>().mention()
    );

    let request = ctx
        .bot
        .http
        .create_message(message.channel_id)
        .content(&content)
        .unwrap()
        .reply(message.id);

    request_for_model(&ctx.bot.http, request).await?;
    Ok(())
}

// Bisaya and Filipino languages are not supported because of complexity
// and Filipino do sometimes mix some words to make it understandable
fn process_name(content: &str) -> Option<String> {
    let Some(index) = get_name_index(content) else {
        return None;
    };

    // assuming that index is within the size of the string
    let mut buffer = String::new();
    let name = content[index..].trim_start();

    // strip any discord's markdown syntax. this will break the bot.
    // repeat this until we have one event only!
    let mut iters = 0;
    loop {
        let mut times = 0;
        if times == 1 || iters > 500 {
            break;
        } else {
            buffer.clear();
        }
        iters += 1;

        let parser = pulldown_cmark::TextMergeStream::new(pulldown_cmark::Parser::new(name));
        for event in parser {
            match event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Paragraph) => {}
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Paragraph) => {}
                pulldown_cmark::Event::Code(data) => {
                    times += 1;
                    buffer.push_str(&data);
                }
                pulldown_cmark::Event::Text(text) => {
                    times += 1;
                    buffer.push_str(&text);
                }
                _ => {}
            }
        }
    }

    if !buffer.is_empty() {
        Some(buffer)
    } else {
        None
    }
}

fn get_name_index(content: &str) -> Option<usize> {
    // I am, My name is, and stuff like that :)
    static I_AM: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(^i|([^\w]i)|([\s]i))(('?m)|( am))").unwrap());

    static MY_NAME_IS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(^(my)|([^\w](my))|([\s](my))) ?name(('s)|( ?is))?").unwrap()
    });

    let content = content.to_lowercase();
    let regex_match = I_AM.find(&content).or_else(|| MY_NAME_IS.find(&content));
    if let Some(index) = regex_match {
        Some(index.end())
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_name() {
        assert_eq!(
            process_name("My name is memothelemo"),
            Some("memothelemo".into())
        );
        assert_eq!(
            process_name("my name is memothelemo"),
            Some("memothelemo".into())
        );

        assert_eq!(process_name("My name ispop"), Some("pop".into()));
        assert_eq!(process_name("My nameispop"), Some("pop".into()));
        assert_eq!(process_name("Mynameispop"), Some("pop".into()));

        assert_eq!(process_name("my name ispop"), Some("pop".into()));
        assert_eq!(process_name("my nameispop"), Some("pop".into()));
        assert_eq!(process_name("mynameispop"), Some("pop".into()));

        // contractions
        assert_eq!(process_name("my name'spop"), Some("pop".into()));
        assert_eq!(process_name("my name'spop"), Some("pop".into()));
        assert_eq!(process_name("myname'spop"), Some("pop".into()));

        // it should strip down markdown
        assert_eq!(process_name("my name is **MEMO**"), Some("MEMO".into()));
    }
}
