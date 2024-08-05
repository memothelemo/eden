use super::exts::{AnyErrorExt, ErrorExt};
use super::{Error, ErrorCategory};

use crate::env::LoadEnvError;
use error_stack::Context;
use thiserror::Error;

/// This trait is applied with types that do not allow for implementation
/// `impl From<Foo> for Error` or you want to have [`Error`] to have meaningful
/// extra error information by using this trait.
pub trait IntoAnonymizedError {
    /// Turns from any error into [`eden_utils::Error`](Error).
    ///
    /// Make sure to put `#[track_caller]` attribute on top of the
    /// function declaration as you may not find where the error
    /// was emitted if not placed.
    fn into_eden_any_error(self) -> Error;
}

/// This trait is applied with types that do not allow for implementation
/// `impl From<Foo> for Error` or you want to have [`Error`] to have meaningful
/// extra error information by using this trait.
///
/// Unlike [`IntoAnyError`] where it returns an anonymized error,
/// [`IntoError::into_eden_error`] only returns a typed error.
pub trait IntoError {
    type Context: Context;

    /// Turns from any error into [`eden_utils::Error`](Error).
    ///
    /// Make sure to put `#[track_caller]` attribute on top of the
    /// function declaration as you may not find where the error
    /// was emitted if not placed.
    fn into_eden_error(self) -> Error<Self::Context>;
}

impl IntoAnonymizedError for std::io::Error {
    #[track_caller]
    fn into_eden_any_error(self) -> Error {
        #[derive(Debug, Error)]
        #[error("I/O error occurred")]
        struct MessageIoError;
        Error::unknown(self).change_context_slient(MessageIoError)
    }
}

impl IntoError for (dotenvy::Error, &'static str) {
    type Context = LoadEnvError;

    #[track_caller]
    fn into_eden_error(self) -> Error<Self::Context> {
        use std::env::VarError;

        let var = self.1;
        match self.0 {
            dotenvy::Error::Io(n) => n.into_eden_any_error().change_context(LoadEnvError),
            dotenvy::Error::EnvVar(VarError::NotPresent) => {
                Error::context(ErrorCategory::Unknown, LoadEnvError)
                    .attach_printable(format!("{var:?} variable is required to set to run Eden"))
            }
            dotenvy::Error::EnvVar(VarError::NotUnicode(..)) => {
                Error::context(ErrorCategory::Unknown, LoadEnvError)
                    .attach_printable(format!("{var:?} variable must contai valid UTF-8 text"))
            }
            _ => unimplemented!(),
        }
    }
}
