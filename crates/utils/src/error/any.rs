use error_stack::Report;
use std::fmt::{Debug, Display};

// Rust won't let me use 'impl ErrorExt for EdenError<()>' because the tuple type
// may be added by Rust devs in later versions. (obviously it won't)
pub struct AnonymizedError;

/// error-stack friendly version of any errors.
pub struct AnyError(String);

impl AnyError {
    #[track_caller]
    pub fn report<T>(error: T) -> Report
    where
        T: std::error::Error,
    {
        Report::new(AnyError(error.to_string())).as_any()
    }
}

impl Debug for AnyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for AnyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl error_stack::Context for AnyError {}
