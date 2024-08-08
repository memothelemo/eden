use eden_utils::error::tags::Suggestion;

pub const NO_LOCAL_GUILD: Suggestion = Suggestion::new(
    "Try checking if your local guild set up exists or configured properly in settings (`bot.local_guild.id`)",
);

#[cfg(test)]
pub const DEV_ENV_NOT_SET_UP: Suggestion = Suggestion::new(
    "Make sure to configure your Eden development environment before running tests",
);
