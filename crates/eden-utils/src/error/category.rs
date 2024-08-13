use strum_macros::Display;
use twilight_model::guild::Permissions;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Display)]
#[must_use]
pub enum ErrorCategory {
    #[strum(to_string = "Guild error")]
    Guild(GuildErrorCategory),
    #[strum(to_string = "User error")]
    User(UserErrorCategory),
    #[default]
    #[strum(to_string = "Error occurred")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UserErrorCategory {
    MissingPermissions,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GuildErrorCategory {
    MissingChannelPermissions(Permissions),
    MissingGuildPermissions(Permissions),
    NotInLocalGuild,
}

impl ErrorCategory {
    #[must_use]
    pub fn is_user_error(&self) -> bool {
        // Self::Guild is considered as human error
        matches!(self, Self::Guild(..) | Self::User(..))
    }
}
