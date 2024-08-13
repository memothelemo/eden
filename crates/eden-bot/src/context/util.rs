use twilight_model::guild::Guild;
use twilight_model::id::{marker::GuildMarker, Id};

use crate::Bot;

#[allow(private_bounds)]
impl Bot {
    #[must_use]
    pub fn is_local_guild(&self, item: &impl GetGuildId) -> bool {
        let guild_id = item.guild_id();
        self.0.settings.bot.local_guild.id == guild_id
    }

    #[must_use]
    pub fn is_sentry_enabled(&self) -> bool {
        self.0.settings.sentry.is_some()
    }
}

trait GetGuildId {
    /// Gets the [guild ID](twilight_model::id::Id) of [`GetGuildId`] implemented object.
    #[doc(hidden)]
    fn guild_id(&self) -> Id<GuildMarker>;
}

impl GetGuildId for Id<GuildMarker> {
    fn guild_id(&self) -> Id<GuildMarker> {
        *self
    }
}

impl GetGuildId for Guild {
    fn guild_id(&self) -> Id<GuildMarker> {
        self.id
    }
}
