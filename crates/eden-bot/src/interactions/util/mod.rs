use crate::interactions::consts;
use eden_utils::error::{exts::*, UserErrorCategory};
use eden_utils::error::{ErrorCategory, GuildErrorCategory};
use eden_utils::sql::SqlErrorExt;
use itertools::Itertools;
use thiserror::Error;
use twilight_model::{channel::message::Embed, http::interaction::InteractionResponseData};
use twilight_util::builder::{embed::EmbedBuilder, InteractionResponseDataBuilder};

pub mod local_guild;

#[derive(Debug, Error)]
#[error("command {0:?} is not implemented")]
pub struct UnknownCommandError(pub(super) String);

/// Builds interaction response data based on [`eden_utils::Error`].
pub fn from_error(
    admin_mode: bool,
    developer_mode: bool,
    error: &eden_utils::Error,
) -> InteractionResponseData {
    let mut embeds = Vec::new();
    if developer_mode {
        render_error_embeds(error, &mut embeds);
        return InteractionResponseDataBuilder::new()
            .content(consts::ERROR_OCCURRED_MESSAGE)
            .embeds(embeds)
            .build();
    }

    // We emit errors differently based on their category
    // - internal
    // - not in local guild
    let embed = match error.get_category() {
        ErrorCategory::Guild(category) => match category {
            GuildErrorCategory::NotInLocalGuild => {
                super::embeds::builders::error("Access denied", None)
                    .description(consts::NOT_ALLOWED_MSG)
                    .build()
            }
            // TODO: Make lacking permissions easier to read
            GuildErrorCategory::MissingChannelPermissions(permissions) => {
                let footer = if admin_mode {
                    consts::ADMIN_MISSING_PERMS_FOOTER
                } else {
                    consts::USER_MISSING_PERMS_FOOTER
                };

                let message = consts::MISSING_CHANNEL_PERMS_MSG
                    .replace("{missing_permissions}", &format!("{permissions:?}"))
                    .replace("{footer}", footer);

                super::embeds::builders::with_emoji('😲', "Oops!")
                    .description(message)
                    .build()
            }
            GuildErrorCategory::MissingGuildPermissions(permissions) => {
                let footer = if admin_mode {
                    consts::ADMIN_MISSING_PERMS_FOOTER
                } else {
                    consts::USER_MISSING_PERMS_FOOTER
                };

                let message = consts::MISSING_GUILD_PERMS_MSG
                    .replace("{missing_permissions}", &format!("{permissions:?}"))
                    .replace("{footer}", footer);

                super::embeds::builders::with_emoji('😲', "Oops!")
                    .description(message)
                    .build()
            }
        },
        ErrorCategory::User(category) => match category {
            UserErrorCategory::MissingGuildPermissions => {
                super::embeds::builders::error("Access denied", None)
                    .description(consts::NOT_ALLOWED_MSG)
                    .build()
            }
        },
        ErrorCategory::Unknown => {
            // unknown is a bit vague
            let msg = if error.is_pool_error() {
                consts::INTERNAL_DB_MSG
            } else {
                consts::INTERNAL_MSG
            };
            super::embeds::builders::error("Something went wrong!", None)
                .description(msg)
                .build()
        }
    };

    InteractionResponseDataBuilder::new()
        .embeds(vec![embed])
        .build()
}

fn render_error_embeds(error: &eden_utils::Error, embeds: &mut Vec<Embed>) {
    // Output includes some of ANSI escape sequences since tracing_error
    // renders out the entire span trace by using the global subscriber
    // set from tracing crate.
    let output = strip_ansi_escapes::strip_str(error.to_string());

    // Split into chunks where each of them has a size is 4000
    // characters long only (96 characters away from Discord's maximum
    // amount of characters for embed descriptions for every embed)
    let chunks = output.chars().chunks(4000);
    let chunks = chunks.into_iter().map(|v| v.collect::<String>());

    for chunk in chunks {
        // Maximum amount of embeds for interaction response
        if embeds.len() == 10 {
            break;
        }

        let embed = EmbedBuilder::new()
            .description(format!("```{chunk}```"))
            .build();

        embeds.push(embed);
    }
}
