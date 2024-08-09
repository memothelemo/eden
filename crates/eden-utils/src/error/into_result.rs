// Instead of copying error_stack's ResultExt, we can control whether
// the error should use `IntoAnyError`/`IntoError` or Error::context_anonymize
// of creating errors.
use core::error::Error as StdError;
use core::result::Result as StdResult;
use error_stack::Context;

use super::into_error::{IntoAnonymizedError, IntoError};
use crate::{Error, ErrorCategory, Result};

type BoxedError = Box<dyn StdError + Send + Sync + 'static>;

pub trait AnonymizeError {
    type Ok;

    /// Anonymizes error without using [`IntoAnyError`].
    fn anonymize_error(self) -> Result<Self::Ok>;
}

pub trait AnonymizeErrorInto {
    type Ok;
    type Error: IntoAnonymizedError;

    /// Anonymizes error using [`IntoAnyError`].
    fn anonymize_error_into(self) -> Result<Self::Ok>;
}

pub trait IntoTypedError {
    type Ok;
    type Error: Context;

    /// Turns into [`Error`] without using [`IntoError`].
    fn into_typed_error(self) -> Result<Self::Ok, Self::Error>;
}

pub trait IntoEdenResult {
    type Ok;
    type Error: Context;

    /// Turns into [`Error`] using [`IntoError`].
    fn into_eden_error(self) -> Result<Self::Ok, Self::Error>;
}

impl<T> AnonymizeError for StdResult<T, BoxedError> {
    type Ok = T;

    #[track_caller]
    fn anonymize_error(self) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(Error::boxed_any(ErrorCategory::Unknown, error)),
        }
    }
}

impl<T, E: IntoAnonymizedError> AnonymizeErrorInto for StdResult<T, E> {
    type Ok = T;
    type Error = E;

    #[track_caller]
    fn anonymize_error_into(self) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_eden_any_error()),
        }
    }
}

impl<T, C: Context> IntoTypedError for StdResult<T, C> {
    type Ok = T;
    type Error = C;

    #[track_caller]
    fn into_typed_error(self) -> Result<T, C> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(Error::context(ErrorCategory::Unknown, error)),
        }
    }
}

impl<T, C: Context, E: IntoError<Context = C>> IntoEdenResult for StdResult<T, E> {
    type Ok = T;
    type Error = C;

    #[track_caller]
    fn into_eden_error(self) -> Result<T, C> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.into_eden_error()),
        }
    }
}
