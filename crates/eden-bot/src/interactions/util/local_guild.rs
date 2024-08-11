use eden_schema::types::PayerApplication;
use eden_utils::error::exts::{IntoEdenResult, ResultExt};
use eden_utils::Result;
use std::borrow::Cow;
use tracing::trace;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::channel::message::Embed;
use twilight_model::id::marker::{ChannelMarker, UserMarker};
use twilight_model::id::{marker::MessageMarker, Id};
use twilight_util::builder::embed::{
    EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource,
};

use crate::errors::RequestHttpError;
use crate::util::image::default_user_avatar;
use crate::Bot;

pub const LEFT_EMOJI: &str = "⬅️";
pub const RIGHT_EMOJI: &str = "➡️";

#[tracing::instrument(skip(bot))]
pub async fn clear_reactions(
    bot: &Bot,
    channel_id: Id<ChannelMarker>,
    message_id: Id<MessageMarker>,
) -> Result<()> {
    trace!("clearing all message reactions");
    bot.http
        .delete_all_reactions(channel_id, message_id)
        .await
        .into_eden_error()
        .change_context(RequestHttpError)
        .attach_printable("could not clear all reactions from a message")?;

    Ok(())
}

#[tracing::instrument(skip(bot))]
pub async fn clear_member_lr_reactions(
    bot: &Bot,
    channel_id: Id<ChannelMarker>,
    message_id: Id<MessageMarker>,
    invoker_id: Id<UserMarker>,
    left: bool,
    right: bool,
) -> Result<()> {
    trace!("clearing member LR reactions");

    if left {
        bot.http
            .delete_reaction(
                channel_id,
                message_id,
                &RequestReactionType::Unicode { name: LEFT_EMOJI },
                invoker_id,
            )
            .await
            .into_eden_error()
            .change_context(RequestHttpError)
            .attach_printable("could not clear left reaction emoji")?;
    }

    if right {
        bot.http
            .delete_reaction(
                channel_id,
                message_id,
                &RequestReactionType::Unicode { name: RIGHT_EMOJI },
                invoker_id,
            )
            .await
            .into_eden_error()
            .change_context(RequestHttpError)
            .attach_printable("could not clear right reaction emoji")?;
    }

    Ok(())
}

#[tracing::instrument(skip(bot))]
pub async fn react_lr_emojis(
    bot: &Bot,
    channel_id: Id<ChannelMarker>,
    message_id: Id<MessageMarker>,
    left: bool,
    right: bool,
) -> Result<()> {
    trace!("reacting navigative emojis");

    if left {
        bot.http
            .create_reaction(
                channel_id,
                message_id,
                &RequestReactionType::Unicode { name: LEFT_EMOJI },
            )
            .await
            .into_eden_error()
            .change_context(RequestHttpError)
            .attach_printable("could not react message with left arrow emoji")?;
    }

    if right {
        bot.http
            .create_reaction(
                channel_id,
                message_id,
                &RequestReactionType::Unicode { name: RIGHT_EMOJI },
            )
            .await
            .into_eden_error()
            .change_context(RequestHttpError)
            .attach_printable("could not react message with right arrow emoji")?;
    }

    Ok(())
}

pub fn render_payer_application_embed(application: &PayerApplication) -> Embed {
    // maybe their icon_url doesn't exist back in MVP version so
    // go with their default avatar instead.
    let icon_url = application
        .icon_url
        .as_deref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| default_user_avatar(application.user_id));

    let author = EmbedAuthorBuilder::new(application.name.clone())
        .icon_url(ImageSource::url(icon_url).unwrap())
        .build();

    let footer = EmbedFooterBuilder::new(format!("Submitted: {}", application.created_at.clone()));
    let content = format!(
        "**Java Username**: `{}`\n**Bedrock Username**: {}\n\n**Reason**:```{}```",
        application.java_username,
        format_value(application.bedrock_username.as_deref()),
        application.answer
    );

    EmbedBuilder::new()
        .author(author)
        .footer(footer)
        .description(content)
        .build()
}

fn format_value(value: Option<&str>) -> Cow<'static, str> {
    if let Some(value) = value {
        Cow::Owned(format!("`{value}`"))
    } else {
        Cow::Borrowed("none")
    }
}
