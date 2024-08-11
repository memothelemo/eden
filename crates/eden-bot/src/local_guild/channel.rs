use eden_utils::error::exts::ResultExt;
use eden_utils::Result;
use tracing::{debug, trace};
use twilight_model::channel::ChannelType;
use twilight_model::guild::{Guild, Permissions};
use twilight_model::id::marker::{ChannelMarker, UserMarker};
use twilight_model::id::Id;
use twilight_util::permission_calculator::PermissionCalculator;

use crate::errors::SendWelcomeMessageError;
use crate::util::has_permission;
use crate::Bot;

/// Attempts to find sendable channels for the bot to send a message with.
pub fn find_sendable_guild_text_channel(bot: &Bot, guild: &Guild) -> Option<Id<ChannelMarker>> {
    let bot_id = bot.application_id().cast::<UserMarker>();

    // Discord should provide member info for the bot.
    let member = guild.members.iter().find(|v| v.user.id == bot_id.cast())?;

    let roles = crate::util::get_member_role_perms(&member.roles, &guild.roles);
    let everyone_role = crate::util::get_everyone_role(&guild)
        .map(|v| v.permissions)
        .unwrap_or_else(Permissions::empty);

    let calculator = PermissionCalculator::new(guild.id, bot_id, everyone_role, &roles);

    let mut sendable_channels = guild
        .channels
        .iter()
        .filter(|channel| {
            let overwrites = channel.permission_overwrites.clone().unwrap_or_default();
            let permissions = calculator.clone().in_channel(channel.kind, &overwrites);

            // we do not want the bot to send something in nsfw channel
            let is_nsfw = channel.nsfw.unwrap_or_default();
            let can_bot_send_message_here = has_permission(permissions, Permissions::SEND_MESSAGES);
            let is_text_channel = channel.kind == ChannelType::GuildText;
            can_bot_send_message_here && is_text_channel && !is_nsfw
        })
        .collect::<Vec<_>>();

    // sort channels by their date
    sendable_channels.sort_by(|a, b| a.id.cmp(&b.id));
    sendable_channels.into_iter().next().map(|v| v.id)
}

#[allow(clippy::expect_used)]
#[tracing::instrument(skip_all, fields(
    channel.id = tracing::field::Empty,
    channel.from_guild = tracing::field::Empty,
    guild.id = %guild.id,
    guild.owner_id = %guild.owner_id,
))]
pub async fn send_welcome_message(bot: &Bot, guild: &Guild) -> Result<(), SendWelcomeMessageError> {
    const MESSAGE: &str = "**Thank you for choosing Eden as your primary Discord bot for your Minecraft server needs!**\n\nYou can setup the bot by running this command and follow instructions from there:\n```/settings setup```";

    // Send this to a text channel or guild owner's DM channel
    let mut is_from_guild = true;
    let channel = match find_sendable_guild_text_channel(bot, guild) {
        Some(channel_id) => channel_id,
        None => {
            trace!("cannot find sendable guild text. creating private channel for guild owner");
            is_from_guild = false;

            // Try to create a DM channel for the guild owner
            let dm_channel = crate::util::http::request_for_model(
                &bot.http,
                bot.http.create_private_channel(guild.owner_id),
            )
            .await
            .change_context(SendWelcomeMessageError)
            .attach_printable("could not create DM channel for the guild owner")?;

            dm_channel.id
        }
    };

    let span = tracing::Span::current();
    if !span.is_disabled() {
        span.record("channel.from_guild", tracing::field::display(is_from_guild));
        span.record("channel.id", tracing::field::display(channel));
    }

    debug!("sending welcome message to channel {channel}");
    let request = bot
        .http
        .create_message(channel)
        .content(MESSAGE)
        .expect("unexpected error while trying to set the message content");

    crate::util::http::request_for_model(&bot.http, request)
        .await
        .change_context(SendWelcomeMessageError)
        .attach_printable_lazy(|| format!("failed to send welcome message to channel {channel}"))
        .attach_printable_lazy(|| format!("guild: {is_from_guild}"))?;

    debug!("sent welcome message to channel {channel}");
    Ok(())
}
