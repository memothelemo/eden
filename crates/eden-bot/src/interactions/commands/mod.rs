use eden_discord_types::commands;
use eden_schema::types::{Admin, User};
use eden_utils::error::{GuildErrorCategory, UserErrorCategory};
use eden_utils::twilight::error::TwilightHttpErrorExt;
use eden_utils::{error::exts::*, Error, ErrorCategory, Result};
use std::fmt::Debug;
use thiserror::Error;
use tracing::{debug, info, trace, warn};
use twilight_interactions::command::{CommandInputData, CommandModel, CreateCommand};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::guild::Permissions;
use twilight_model::id::marker::UserMarker;
use twilight_util::permission_calculator::PermissionCalculator;

use crate::errors::RegisterCommandsError;
use crate::interactions::tags::{CheckPermsInvokerTag, LackingPermissionsTag};
use crate::interactions::LocalGuildContext;
use crate::util::http::request_for_model;
use crate::Bot;

mod context;
mod local_guild;
mod ping;

pub use self::context::*;

#[allow(async_fn_in_trait)]
pub trait RunCommand: CreateCommand + CommandModel + Debug {
    /// Attempts to runs the command.
    ///
    /// This function assumes that you already sent the interaction
    /// response from Discord.
    ///
    /// If not done, the interaction will be considered as invalid
    /// and will result an error to the end user/invoker.
    async fn run(&self, ctx: &CommandContext) -> Result<()>;

    /// Required invoker's guild permissions to perform this command.
    fn user_permissions(&self) -> Permissions {
        Permissions::empty()
    }

    /// Required bot guild permissions to perform this command.
    ///
    /// Usually, the default is empty means that no permissions
    /// needed for the bot to perform something from this command.
    fn guild_permissions(&self) -> Permissions {
        Permissions::empty()
    }

    /// Required bot channel permissions to perform this command.
    ///
    /// Usually, the default is empty means that no permissions
    /// needed for the bot to perform something from this command.
    fn channel_permissions(&self) -> Permissions {
        Permissions::empty()
    }
}

pub async fn handle(ctx: CommandContext) -> Result<()> {
    debug!("received command: {:?}", ctx.data.name);

    macro_rules! match_commands {
        ($ctx:expr, $data:expr, [ $($command:ty),* $(,)? ]) => (match $ctx.data.name.as_str() {
            $( <$command>::NAME => handle_command::<$command>(&$ctx, $data).await, )*
            _ => $ctx.unimplemented_cmd(),
        });
    }

    let input: CommandInputData<'_> = ctx.data.clone().into();
    let name = ctx.command_name();
    let result = match_commands!(
        ctx,
        input,
        [
            commands::local_guild::PayerCommand,
            commands::local_guild::SettingsCommand,
            commands::Ping
        ]
    );

    let Err(error) = result else {
        trace!("successfully ran command {name:?}");
        return Ok(());
    };

    let is_admin = error
        .get_attached_any::<CheckPermsInvokerTag>()
        .next()
        .map(|v| v.is_admin)
        .unwrap_or_default();

    let mut conn = ctx.bot.db_read().await?;
    let user = User::get_or_insert(&mut conn, ctx.invoker_id()).await?;
    let data = super::util::from_error(is_admin, user.developer_mode, &error);

    // log error messages for non-user errors.
    if !error.get_category().is_user_error() {
        warn!(%error, "failed to run command {name:?}");
    }

    ctx.respond(data)
        .await
        .attach_printable("could not respond command while trying to send error message")?;

    Ok(())
}

pub async fn register(bot: &Bot) -> Result<(), RegisterCommandsError> {
    use eden_discord_types::commands;
    macro_rules! create_cmds {
        [ $($command:ty),* $(,)? ] => {
            vec!{$( <$command as CreateCommand>::create_command().into(), )*}
        };
    }
    let interaction = bot.interaction();

    let global_commands = create_cmds![commands::Ping];
    let local_guild_commands = create_cmds![
        commands::local_guild::PayerCommand,
        commands::local_guild::SettingsCommand
    ];

    let total_groups = global_commands.len() + local_guild_commands.len();
    let local_guild_id = bot.settings.bot.local_guild.id;

    debug!(
        "setting global commands with {} command group(s)",
        global_commands.len()
    );
    interaction
        .set_global_commands(&global_commands)
        .await
        .into_typed_error()
        .change_context(RegisterCommandsError)?;

    debug!(
        "setting guild ({local_guild_id}) commands with {} command group(s)",
        local_guild_commands.len()
    );
    interaction
        .set_guild_commands(local_guild_id, &local_guild_commands)
        .await
        .into_typed_error()
        .change_context(RegisterCommandsError)?;

    info!("registered {total_groups} command group(s)");
    Ok(())
}

#[derive(Debug, Error)]
enum LackingBotPermissions {
    #[error("bot lacked channel permissions to use the command {0:?}")]
    Channel(String),
    #[error("bot lacked guild permissions to use the command {0:?}")]
    Guild(String),
}

#[derive(Debug, Error)]
#[error("user lacked permissions to use the command {0:?}")]
struct LackingUserPermissions(String);

#[tracing::instrument(skip_all)]
async fn fetch_guild_and_channel_permissions(
    ctx: &LocalGuildContext<'_, CommandData>,
    needs_channel_info: bool,
) -> Result<(Permissions, Option<Permissions>)> {
    let cache = &ctx.bot.cache;
    let bot_id = ctx.bot.application_id().cast::<UserMarker>();

    let guild = request_for_model(&ctx.bot.http, ctx.bot.http.guild(ctx.guild_id)).await?;
    let everyone_role = crate::util::get_everyone_role(&guild)
        .map(|v| v.permissions)
        .unwrap_or_else(Permissions::empty);

    // Consider trying from cache first?
    let member_roles = if let Some(member) = cache.member(ctx.guild_id, bot_id) {
        trace!("cache hit, got member info from cache");
        member.roles().to_vec()
    } else {
        trace!("cache miss, getting member info from Discord API");
        request_for_model(
            &ctx.bot.http,
            ctx.bot.http.guild_member(ctx.guild_id, bot_id),
        )
        .await?
        .roles
    };

    let mut channel_kind = None;
    let mut overwrites = None;

    if let Some(channel) = cache.channel(ctx.channel_id) {
        trace!("cache hit, got channel info from cache");

        let overwrites_data = channel.permission_overwrites.clone().unwrap_or_default();
        channel_kind = Some(channel.kind);
        overwrites = Some(overwrites_data);
    } else if needs_channel_info {
        // do not request for channels stuff if it is not really required anyways.
        trace!("cache miss, getting channel info from Discord API");

        let channel =
            request_for_model(&ctx.bot.http, ctx.bot.http.channel(ctx.channel_id)).await?;

        channel_kind = Some(channel.kind);
        overwrites = channel.permission_overwrites;
    } else {
        trace!("cache miss, not getting channel info from Discord API");
    }

    let member_roles = crate::util::get_member_role_perms(&member_roles, &guild.roles);
    trace!(?member_roles, ?everyone_role);
    let calculator = PermissionCalculator::new(ctx.guild_id, bot_id, everyone_role, &member_roles);

    let guild = calculator.root();
    let channel = channel_kind
        .zip(overwrites)
        .map(|(channel_kind, overwrites)| calculator.in_channel(channel_kind, &overwrites));

    Ok((guild, channel))
}

#[allow(clippy::unwrap_used)]
#[tracing::instrument(skip_all, fields(
    command.channel_permissions = ?command.channel_permissions(),
    command.guild_permissions = ?command.guild_permissions(),
    bot.channel_permissions = tracing::field::Empty,
    bot.guild_permissions = tracing::field::Empty,
))]
async fn check_bot_guild_permissions<T: CommandModel + RunCommand>(
    command: &T,
    ctx: &LocalGuildContext<'_, CommandData>,
) -> Result<()> {
    let channel_required = command.channel_permissions();
    let guild_required = command.guild_permissions();

    // To save HTTP request quota from Discord, absolutely check if channel_required
    // and guild_required is not empty.
    if channel_required.is_empty() && guild_required.is_empty() {
        return Ok(());
    }

    trace!("fetching bot's guild and channel permissions");
    let result = fetch_guild_and_channel_permissions(ctx, !channel_required.is_empty()).await;
    if let Some(info) = result.discord_http_error_info() {
        // somehow the API cannot give channel info apparently
        if info.has_missing_access() && !channel_required.is_empty() {
            trace!("got missing access error while trying to get channel info");
            return Err(Error::context_anonymize(
                ErrorCategory::Guild(GuildErrorCategory::MissingChannelPermissions(
                    Permissions::VIEW_CHANNEL,
                )),
                LackingBotPermissions::Channel(ctx.command_name()),
            ));
        }
    }

    let (current_guild_permissions, current_channel_permissions) = result?;
    let span = tracing::Span::current();
    if !span.is_disabled() {
        span.record(
            "bot.guild_permissions",
            tracing::field::debug(&current_guild_permissions),
        );
        span.record(
            "bot.channel_permissions",
            tracing::field::debug(&current_channel_permissions),
        );
    }
    trace!("received permissions");

    // Go with channel first perhaps
    if !channel_required.is_empty()
        && !current_channel_permissions
            .unwrap()
            .contains(channel_required)
    {
        let tag =
            LackingPermissionsTag::new(current_channel_permissions.unwrap(), channel_required);

        return Err(Error::context_anonymize(
            ErrorCategory::Guild(GuildErrorCategory::MissingChannelPermissions(
                tag.calculated(),
            )),
            LackingBotPermissions::Channel(ctx.command_name()),
        ))
        .attach(tag);
    }

    // Go with channel first perhaps
    if !current_guild_permissions.contains(guild_required) {
        let tag = LackingPermissionsTag::new(current_guild_permissions, guild_required);
        Err(Error::context_anonymize(
            ErrorCategory::Guild(GuildErrorCategory::MissingGuildPermissions(
                tag.calculated(),
            )),
            LackingBotPermissions::Guild(ctx.command_name()),
        ))
        .attach(tag)
    } else {
        Ok(())
    }
}

#[tracing::instrument(skip_all, fields(
    command.user_permissions = tracing::field::Empty,
    invoker.permissions = tracing::field::Empty,
))]
async fn check_user_guild_permissions<T: CommandModel + RunCommand>(
    command: &T,
    ctx: &LocalGuildContext<'_, CommandData>,
) -> Result<()> {
    // If the command actually requires admin permissions, we need to
    // check to the database first to save HTTP request quota to Discord
    let mut user_permissions = Permissions::empty();
    let required = command.user_permissions();
    let span = tracing::Span::current();
    if !span.is_disabled() {
        span.record("command.user_permissions", tracing::field::debug(&required));
    }

    if required.contains(Permissions::ADMINISTRATOR) {
        trace!("this command requires admin permissions. checking if the user is an admin from the database...");
        let mut conn = ctx.bot.db_read().await?;
        if Admin::from_id(&mut conn, ctx.author.id).await?.is_some() {
            user_permissions = Permissions::ADMINISTRATOR;
        }
    } else if !required.is_empty() {
        trace!("fetching user's guild permissions");
        user_permissions = ctx.permissions().await?;
    }

    if !span.is_disabled() {
        span.record(
            "invoker.permissions",
            tracing::field::debug(&user_permissions),
        );
    }

    if !user_permissions.contains(required) {
        Err(Error::context_anonymize(
            ErrorCategory::User(UserErrorCategory::MissingPermissions),
            LackingUserPermissions(ctx.command_name()),
        ))
        .attach(LackingPermissionsTag::new(user_permissions, required))
    } else {
        Ok(())
    }
}

async fn handle_command<'a, T: CommandModel + RunCommand>(
    ctx: &CommandContext,
    data: CommandInputData<'a>,
) -> Result<()> {
    trace!("parsing command for {:?}", T::NAME);
    let command = T::from_interaction(data)
        .into_typed_error()
        .attach_printable_lazy(|| {
            format!("could not parse {:?} command from interaction", T::NAME)
        })?;

    // TODO: guild permission checks
    let guild_ctx = LocalGuildContext::from_ctx(ctx).await.ok();
    if let Some(ctx) = guild_ctx {
        let permissions = ctx.member.permissions.unwrap_or_else(Permissions::empty);
        let tag = CheckPermsInvokerTag {
            is_admin: permissions.contains(Permissions::ADMINISTRATOR),
        };

        check_user_guild_permissions(&command, &ctx)
            .await
            .attach(tag)?;

        check_bot_guild_permissions(&command, &ctx)
            .await
            .attach(tag)?;
    }

    command.run(ctx).await
}
