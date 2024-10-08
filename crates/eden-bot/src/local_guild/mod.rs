use eden_schema::forms::InsertAdminForm;
use eden_schema::types::{Admin, GuildSettings};
use eden_utils::{error::exts::*, Result};
use tracing::{debug, info, trace, warn};
use twilight_model::guild::{Guild, Permissions};
use twilight_util::permission_calculator::PermissionCalculator;

use crate::errors::{SetupLocalGuildError, UpdateLocalGuildAdminsError};
use crate::Bot;

pub mod channel;

/// Updates the list of administrators from the local guild.
#[tracing::instrument(skip_all, fields(guild.id = %guild.id))]
pub async fn update_admins(bot: &Bot, guild: &Guild) -> Result<(), UpdateLocalGuildAdminsError> {
    debug!("updating local guild administrators");

    let mut conn = bot
        .db_write()
        .await
        .change_context(UpdateLocalGuildAdminsError)?;

    let everyone_role = crate::util::get_everyone_role(&guild)
        .map(|v| v.permissions)
        .unwrap_or_else(Permissions::empty);

    let mut after = None;
    let mut guild_admins = Vec::new();
    loop {
        let mut request = bot
            .http
            .guild_members(guild.id)
            .limit(500)
            .expect("unexpected error when setting limit to 500");

        if let Some(after) = after.take() {
            request = request.after(after);
        }

        trace!(?after, "fetching batch of guild members");
        let members = crate::util::http::request_for_list(&bot.http, request)
            .await
            .change_context(UpdateLocalGuildAdminsError)
            .attach_printable("failed to fetch all guild members")?;

        trace!("got response with {} member(s)", members.len());
        for member in members.iter() {
            let roles = crate::util::get_member_role_perms(&member.roles, &guild.roles);
            let user_id = member.user.id;
            let is_admin = {
                PermissionCalculator::new(guild.id, user_id, everyone_role, &roles)
                    .owner_id(guild.owner_id)
                    .root()
                    .contains(Permissions::ADMINISTRATOR)
                    && !member.user.bot
            };

            trace!(user.id = ?user_id, %is_admin, ?roles, ?everyone_role);
            if !is_admin {
                continue;
            }

            debug!("found local guild admin with user {user_id}");
            guild_admins.push(member.user.clone());
        }

        if members.len() != 500 {
            trace!(members.len = ?members.len(), "fetch stopped");
            break;
        }
        after = members.iter().last().map(|v| v.user.id);
    }

    if guild_admins.is_empty() {
        warn!("local guild {} has no guild administrators. please have one guild administrator to setup the Eden bot", guild.id);
        return Ok(());
    }

    for admin in guild_admins.iter() {
        trace!("initializing admin data for user {}", admin.id);
        let form = InsertAdminForm::builder()
            .id(admin.id)
            .name(Some(&admin.name))
            .build();

        Admin::upsert(&mut conn, form)
            .await
            .change_context(UpdateLocalGuildAdminsError)
            .attach_printable_lazy(|| format!("could not upsert admin data for {}", admin.id))?;
    }

    conn.commit()
        .await
        .anonymize_error_into()
        .change_context(UpdateLocalGuildAdminsError)
        .attach_printable("could not commit database transaction")?;

    info!("loaded {} local guild admin(s)", guild_admins.len());
    Ok(())
}

/// Sets up local guild with initial data.
#[allow(clippy::expect_used)]
#[tracing::instrument(skip_all, fields(guild.id = %guild.id))]
pub async fn setup(bot: &Bot, guild: &Guild) -> Result<(), SetupLocalGuildError> {
    assert!(
        bot.is_local_guild(guild),
        "tried to initialize local guild with non-local guild"
    );

    debug!("setting up local guild {}", guild.id);
    let mut conn = bot.db_write().await.change_context(SetupLocalGuildError)?;
    let settings = GuildSettings::upsert(&mut conn, guild.id)
        .await
        .change_context(SetupLocalGuildError)
        .attach_printable("could not load guild settings")?;

    conn.commit()
        .await
        .anonymize_error_into()
        .change_context(SetupLocalGuildError)
        .attach_printable("could not commit database transaction")?;

    let is_initial_setup = settings.updated_at.is_none();
    if is_initial_setup {
        debug!(?settings, "created local guild settings");
    } else {
        debug!(?settings, "loaded local guild settings");
    }

    // Check if alert_channel_id exists, otherwise warn the user
    let alert_channel_exists = guild
        .channels
        .iter()
        .any(|v| v.id == bot.settings.bot.local_guild.alert_channel_id);

    if !alert_channel_exists {
        warn!("Eden detects that your configured alert channel does not exists and it may not work as intended!\n\n{}", crate::suggestions::NO_ALERT_CHANNEL_ID.as_str());
    }

    update_admins(bot, guild)
        .await
        .change_context(SetupLocalGuildError)?;

    if is_initial_setup {
        self::channel::send_welcome_message(bot, guild)
            .await
            .change_context(SetupLocalGuildError)?;
    }

    Ok(())
}
