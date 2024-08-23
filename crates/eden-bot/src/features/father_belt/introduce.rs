use eden_utils::{twilight::error::TwilightHttpErrorExt, Result};
use regex::Regex;
use rustrict::Censor;
use std::sync::LazyLock;
use tracing::{instrument, trace, warn};
use twilight_mention::Mention;
use twilight_model::channel::Message;
use twilight_model::id::marker::UserMarker;
use url::Url;

use crate::events::EventContext;
use crate::util::http::request_for_model;

#[instrument(skip_all)]
pub async fn on_trigger(ctx: &EventContext, message: &Message) {
    if message.guild_id.is_none() {
        return;
    }

    let Some((name, index)) = get_supposed_name(&message.content) else {
        return;
    };

    if !is_name_valid(&name, &message.content, index) {
        return;
    }

    trace!("relying back introduction message");
    if let Err(error) = respond(ctx, &message, &name).await {
        let has_missing_access = error
            .discord_http_error_info()
            .map(|v| v.has_missing_access())
            .unwrap_or_default();

        if !has_missing_access {
            warn!(%error, "could not respond back introduction message to the user");
        }
    }
}

// We don't want to let Eden say "Hi <swear word>" when the user said that so.
//
// By the way, this is inspired by Dad Bot#2189 made by alekeagle
#[tracing::instrument(skip_all)]
async fn respond(ctx: &EventContext, message: &Message, name: &str) -> Result<()> {
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

// From: https://github.com/memothelemo/eden/issues/9
fn is_name_valid(processed: &str, original_content: &str, name_index: usize) -> bool {
    static DISCORD_MENTION_TAG: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<@[0-9]+>").unwrap());

    if let Some(captures) = DISCORD_MENTION_TAG.captures(processed) {
        // Do not return it as Some.
        //
        // Users will get advantage of the bug that allows the bot to ping the
        // server administrator or any role and we don't want let it happend.
        for id in 0..captures.len() {
            if captures.get(id).is_some() {
                return false;
            }
        }
    }

    // Checking if the finalized buffer comes from a URL part of the message.
    // Related to issue #9.
    let (left, right) = original_content.split_at(name_index);
    let left = left.split_whitespace().last().unwrap_or("");
    let right = right.split_whitespace().next().unwrap_or("");

    let mut part = String::new();
    part.push_str(left);
    part.push_str(right);

    Url::parse(&part).is_err()
}

// Bisaya and Filipino languages are not supported because of complexity
// and Filipino do sometimes mix some words to make it understandable
fn get_supposed_name(content: &str) -> Option<(String, usize)> {
    // I am... My name is...
    static I_AM: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(^i|([^\w]i)|([\s]i))(('?m)|( am))").unwrap());

    static MY_NAME_IS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(^(my)|([^\w](my))|([\s](my))) ?name(('s)|( ?is))?").unwrap()
    });

    let lowercased_content = content.to_lowercase();
    let index = I_AM
        .find(&lowercased_content)
        .or_else(|| MY_NAME_IS.find(&lowercased_content))?
        .end();

    // assuming that index is within the size of the string
    let mut buffer = String::new();
    let name = &content[index..].trim_start();

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
        Some((buffer, index))
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use twilight_model::id::Id;

    #[test]
    fn test_issue_9_fix() {
        let user_id = Id::<UserMarker>::new(1234567890);
        let message = format!("I'm {}", user_id.mention());
        let (name, index) = get_supposed_name(&message).unwrap();
        assert!(!is_name_valid(&name, &message, index));

        let message = "here is my face: https://example.com/image.png";
        let (name, index) = get_supposed_name(message).unwrap();
        assert!(!is_name_valid(&name, message, index));

        let message = "https://example.com/image.png";
        let (name, index) = get_supposed_name(message).unwrap();
        assert!(!is_name_valid(&name, message, index));

        let message = "I'm a guy";
        let (name, index) = get_supposed_name(message).unwrap();
        assert!(is_name_valid(&name, message, index));

        // real world scenario, this is the exact link caused the issue #9
        let message =
            "https://media.discordapp.net/attachments/123/456/imagdse0.gif?ex=6&is=66&hm=4f9dd&";

        let (name, index) = get_supposed_name(message).unwrap();
        assert!(!is_name_valid(&name, message, index));
    }

    #[test]
    fn test_find_name() {
        assert_eq!(
            get_supposed_name("My name is memothelemo").map(|(a, ..)| a),
            Some("memothelemo".into())
        );
        assert_eq!(
            get_supposed_name("my name is memothelemo").map(|(a, ..)| a),
            Some("memothelemo".into())
        );

        assert_eq!(
            get_supposed_name("My name ispop").map(|(a, ..)| a),
            Some("pop".into())
        );
        assert_eq!(
            get_supposed_name("My nameispop").map(|(a, ..)| a),
            Some("pop".into())
        );
        assert_eq!(
            get_supposed_name("Mynameispop").map(|(a, ..)| a),
            Some("pop".into())
        );

        assert_eq!(
            get_supposed_name("my name ispop").map(|(a, ..)| a),
            Some("pop".into())
        );
        assert_eq!(
            get_supposed_name("my nameispop").map(|(a, ..)| a),
            Some("pop".into())
        );
        assert_eq!(
            get_supposed_name("mynameispop").map(|(a, ..)| a),
            Some("pop".into())
        );

        // contractions
        assert_eq!(
            get_supposed_name("my name'spop").map(|(a, ..)| a),
            Some("pop".into())
        );
        assert_eq!(
            get_supposed_name("my name'spop").map(|(a, ..)| a),
            Some("pop".into())
        );
        assert_eq!(
            get_supposed_name("myname'spop").map(|(a, ..)| a),
            Some("pop".into())
        );

        // it should strip down markdown
        assert_eq!(
            get_supposed_name("my name is **MEMO**").map(|(a, ..)| a),
            Some("MEMO".into())
        );
    }
}