use strum_macros::Display;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Display)]
#[must_use]
#[non_exhaustive]
pub enum ErrorCategory {
    #[strum(to_string = "Guild error")]
    Guild(GuildErrorCategory),
    #[strum(to_string = "User error")]
    User,
    #[default]
    #[strum(to_string = "Error occurred")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GuildErrorCategory {
    NotInLocalGuild,
}

impl ErrorCategory {
    #[must_use]
    pub fn is_user_error(&self) -> bool {
        // Self::Guild is considered as human error
        matches!(self, Self::Guild(..) | Self::User)
    }
}
