use super::{Error as EdenError, IntoError};
use crate::error::ErrorCategory;

use error_stack::{Context, Report};
use std::fmt::{Debug, Display};
use thiserror::Error;

/// error-stack friendly version of [`std::io::Error`].
struct CustomIoError(std::io::Error);

impl IntoError for std::io::Error {
    fn into_eden_error(self) -> EdenError {
        #[derive(Debug, Error)]
        #[error("I/O error occurred")]
        struct MessageIoError;

        let report = Report::new(CustomIoError(self)).change_context(MessageIoError);
        EdenError::report_anonymize(ErrorCategory::Unknown, report)
    }
}

impl<T: Context> IntoError for Report<T> {
    fn into_eden_error(self) -> EdenError {
        EdenError::report_anonymize(ErrorCategory::Unknown, self)
    }
}

impl Debug for CustomIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for CustomIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl error_stack::Context for CustomIoError {}
