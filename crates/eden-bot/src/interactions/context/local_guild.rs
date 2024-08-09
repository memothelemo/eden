use eden_schema::types::{GuildSettings, GuildSettingsRow};
use eden_utils::error::GuildErrorCategory;
use eden_utils::{Error, ErrorCategory, Result};
use std::fmt::Debug;
use std::ops::Deref;
use thiserror::Error;
use tracing::trace;
use twilight_model::guild::{PartialMember, Permissions};
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;
use twilight_model::user::User;
use twilight_util::permission_calculator::PermissionCalculator;

use super::InteractionContext;

/// Extension of [`InteractionContext`] but it contains special local guild
/// data. This allows for easier access like local guild settings.
pub struct LocalGuildContext<'a, T> {
    /// User that invoked the interaction.
    pub author: &'a User,

    /// Local guild's ID.
    pub guild_id: Id<GuildMarker>,

    /// Guild member object of the invoker.
    pub member: &'a PartialMember,

    /// Local guild settings
    pub settings: GuildSettingsRow,

    /// Inner data of [`LocalGuildContext`].
    pub inner: &'a InteractionContext<T>,
}

impl<'a, T> Debug for LocalGuildContext<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalGuildContext")
            .field("author", &self.author.id)
            .field("channel_id", &self.inner.channel_id)
            .field("settings", &self.settings)
            .finish()
    }
}

#[derive(Debug, Error)]
#[error("unexpected local guild only interaction was invoked in non-local guild")]
pub struct NotInLocalGuildError;

impl<'a, T> LocalGuildContext<'a, T> {
    /// Create a new [`GuildInteractionContext`] from [interaction context](InteractionContext).
    ///
    /// This function assumes that the interaction given was invoked
    /// from a local guild.
    pub async fn from_ctx(ctx: &'a InteractionContext<T>) -> Result<Self> {
        trace!(?ctx.interaction.guild_id, ?ctx.interaction.member);

        let Some(guild_id) = ctx.interaction.guild_id.as_ref() else {
            return Err(Error::context_anonymize(
                ErrorCategory::Guild(GuildErrorCategory::NotInLocalGuild),
                NotInLocalGuildError,
            ));
        };

        if !ctx.bot.is_local_guild(guild_id) {
            return Err(Error::context_anonymize(
                ErrorCategory::Guild(GuildErrorCategory::NotInLocalGuild),
                NotInLocalGuildError,
            ));
        }

        let Some(member) = ctx.interaction.member.as_ref() else {
            return Err(Error::context_anonymize(
                ErrorCategory::Guild(GuildErrorCategory::NotInLocalGuild),
                NotInLocalGuildError,
            ));
        };

        let Some(author) = member.user.as_ref() else {
            return Err(Error::context_anonymize(
                ErrorCategory::Guild(GuildErrorCategory::NotInLocalGuild),
                NotInLocalGuildError,
            ));
        };

        let mut conn = ctx.bot.db_read().await?;
        let settings = GuildSettings::upsert(&mut conn, *guild_id).await?;
        trace!(?settings, "got local guild settings");

        Ok(Self {
            author,
            guild_id: *guild_id,
            member,
            settings,
            inner: ctx,
        })
    }

    /// Resolves invoker's local guild member permissions.
    #[must_use]
    pub async fn permissions(&self) -> Result<Permissions> {
        let cache = self.bot.cache.permissions();
        if let Some(permissions) = cache.root(self.author.id, self.guild_id).ok() {
            return Ok(permissions);
        }

        // TODO: Find a way to reduce this request
        let guild = crate::util::http::request_for_model(
            &self.bot.http,
            self.bot.http.guild(self.guild_id),
        )
        .await?;

        let everyone_role = crate::util::get_everyone_role(&guild)
            .map(|v| v.permissions)
            .unwrap_or_else(Permissions::empty);

        let member_roles = crate::util::get_member_role_perms(&self.member.roles, &guild.roles);
        let calculator =
            PermissionCalculator::new(self.guild_id, self.author.id, everyone_role, &member_roles);

        Ok(calculator.root())
    }
}

impl<'a, T> Deref for LocalGuildContext<'a, T> {
    type Target = InteractionContext<T>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

macro_rules! record_local_guild_ctx {
    ($ctx:expr) => {{
        let span = tracing::Span::current();
        if !span.is_disabled() {
            span.record("ctx", tracing::field::debug(&$ctx));
        }
    }};
}
pub(crate) use record_local_guild_ctx;
