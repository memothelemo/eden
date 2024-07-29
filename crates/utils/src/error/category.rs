use strum_macros::Display;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Display)]
#[must_use]
#[non_exhaustive]
pub enum ErrorCategory {
    #[strum(to_string = "Guild error")]
    Guild,
    #[strum(to_string = "User error")]
    User,
    #[default]
    #[strum(to_string = "Error occurred")]
    Unknown,
}
