use error_stack::{iter::RequestRef, Context};
use std::error::Error as StdError;
use std::fmt;
use std::marker::PhantomData;
use std::result::Result as StdResult;
use tracing_error::SpanTrace;

use super::{into_error::IntoAnyError, IntoError};
use crate::error::{Error as EdenError, ErrorCategory, Result};

/// Extension trait for [`Result`](core::result::Result) to conveniently
/// transform from any error into a typed [`Error`](EdenError).
///
/// Unlike [`ResultExtInto`] which it can be utilized if the error from the
/// [`Result`](core::result::Result) type is implemented with [`IntoError`],
/// [`IntoError::into_eden_error`] will not be called and instead creates a new
/// report that lacks contextual information.
pub trait ResultExt {
    type Ok;
    type Context;

    fn anonymize_error(self) -> Result<Self::Ok>;

    fn attach<A>(self, attachment: A) -> Result<Self::Ok, Self::Context>
    where
        A: Send + Sync + 'static;

    fn attach_lazy<A, F>(self, attachment: F) -> Result<Self::Ok, Self::Context>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A;

    fn attach_printable<A>(self, attachment: A) -> Result<Self::Ok, Self::Context>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static;

    fn attach_printable_lazy<A, F>(self, attachment: F) -> Result<Self::Ok, Self::Context>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A;

    fn category(self, category: ErrorCategory) -> Result<Self::Ok, Self::Context>;

    fn change_context<P>(self, context: P) -> Result<Self::Ok, P>
    where
        P: Context;

    fn change_context_lazy<P, F>(self, context: F) -> Result<Self::Ok, P>
    where
        P: Context,
        F: FnOnce() -> P;
}

// Rust won't allow me to implement this:
// impl<T, C: Context, E: IntoError<Context = C>> ResultExt for StdResult<T, E>
//
/// Extension trait for [`Result`](core::result::Result) to conveniently
/// transform from any error into a typed [`Error`](EdenError).
///
/// Unlike [`ResultExt`] which it can be utilized if the error from the
/// [`Result`](core::result::Result) type is implemented with [`IntoError`],
/// [`IntoError::into_eden_error`] will be called and it creates a report
/// that may have contextual information (depending on the error type you're using).
///
/// Additionally, all functions from [`ResultExt`] are named with `into` suffix
/// to differentiate functions from [`ResultExt`].
pub trait ResultExtInto {
    type Ok;
    type Context;

    // Yes. The naming is a bit awkward.
    fn anonymize_error_into(self) -> Result<Self::Ok>;

    fn attach_into<A>(self, attachment: A) -> Result<Self::Ok, Self::Context>
    where
        A: Send + Sync + 'static;

    fn attach_lazy_into<A, F>(self, attachment: F) -> Result<Self::Ok, Self::Context>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A;

    fn attach_printable_into<A>(self, attachment: A) -> Result<Self::Ok, Self::Context>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static;

    fn attach_printable_lazy_into<A, F>(self, attachment: F) -> Result<Self::Ok, Self::Context>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A;

    fn category_into(self, category: ErrorCategory) -> Result<Self::Ok, Self::Context>;

    fn change_context_into<P>(self, context: P) -> Result<Self::Ok, P>
    where
        P: Context;

    fn change_context_lazy_into<P, F>(self, context: F) -> Result<Self::Ok, P>
    where
        P: Context,
        F: FnOnce() -> P;
}

/// Extension trait for [`Result`](core::result::Result) to conveniently
/// transform from any error into an anonymized [`Error`](EdenError).
///
/// Unlike [`AnyResultExtInto`] which it can be utilized if the error from the
/// [`Result`](core::result::Result) type is implemented with [`IntoAnyError`],
/// [`IntoAnyError::into_eden_any_error`], boxed error (`Box<dyn Error>`) or anonymized
/// [`Error`](EdenError)  will not be called and instead creates a new error that
/// lacks contextual information.
pub trait AnyResultExt {
    type Ok;

    fn attach<A>(self, attachment: A) -> Result<Self::Ok>
    where
        A: Send + Sync + 'static;

    fn attach_lazy<A, F>(self, attachment: F) -> Result<Self::Ok>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A;

    fn attach_printable<A>(self, attachment: A) -> Result<Self::Ok>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static;

    fn attach_printable_lazy<A, F>(self, attachment: F) -> Result<Self::Ok>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A;

    fn category(self, category: ErrorCategory) -> Result<Self::Ok>;

    fn change_context<P>(self, context: P) -> Result<Self::Ok, P>
    where
        P: Context;

    fn push_context<P>(self, context: P) -> Result<Self::Ok>
    where
        P: Context;

    fn push_context_lazy<P, F>(self, context: F) -> Result<Self::Ok>
    where
        P: Context,
        F: FnOnce() -> P;

    fn change_context_lazy<P, F>(self, context: F) -> Result<Self::Ok, P>
    where
        P: Context,
        F: FnOnce() -> P;
}

/// Extension trait for [`Result`](core::result::Result) to conveniently
/// transform from any error into an anonymized [`Error`](EdenError).
///
/// Unlike [`AnyResultExtInto`] which it can be utilized if the error from the
/// [`Result`](core::result::Result) type is implemented with [`IntoAnyError`],
/// [`IntoAnyError::into_eden_any_error`], boxed error (`Box<dyn Error>`) or anonymized
/// [`Error`](EdenError) will be called and it creates a report that may have contextual
/// information (depending on the error type you're using).
///
/// Additionally, all functions from [`AnyResultExt`] are named with `into` suffix
/// to differentiate functions from [`AnyResultExt`].
pub trait AnyResultExtInto {
    type Ok;

    fn anonymize_error_into(self) -> Result<Self::Ok>;

    fn attach_into<A>(self, attachment: A) -> Result<Self::Ok>
    where
        A: Send + Sync + 'static;

    fn attach_lazy_into<A, F>(self, attachment: F) -> Result<Self::Ok>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A;

    fn attach_printable_into<A>(self, attachment: A) -> Result<Self::Ok>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static;

    fn attach_printable_lazy_into<A, F>(self, attachment: F) -> Result<Self::Ok>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A;

    fn category_into(self, category: ErrorCategory) -> Result<Self::Ok>;

    fn change_context_into<P>(self, context: P) -> Result<Self::Ok, P>
    where
        P: Context;

    fn push_context_into<P>(self, context: P) -> Result<Self::Ok>
    where
        P: Context;

    fn push_context_lazy_into<P, F>(self, context: F) -> Result<Self::Ok>
    where
        P: Context,
        F: FnOnce() -> P;

    fn change_context_lazy_into<P, F>(self, context: F) -> Result<Self::Ok, P>
    where
        P: Context,
        F: FnOnce() -> P;
}

// Boxed errors to Eden errors will be anonymous automatically.
impl<T> AnyResultExt for StdResult<T, Box<dyn StdError + Send + Sync + 'static>> {
    type Ok = T;

    #[track_caller]
    fn attach<A>(self, attachment: A) -> Result<T>
    where
        A: Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::boxed_any(ErrorCategory::Unknown, error).attach(attachment))
            }
        }
    }

    #[track_caller]
    fn attach_lazy<A, F>(self, attachment: F) -> Result<T>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::boxed_any(ErrorCategory::Unknown, error).attach(attachment()))
            }
        }
    }

    #[track_caller]
    fn attach_printable<A>(self, attachment: A) -> Result<T>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::boxed_any(ErrorCategory::Unknown, error)
                    .attach_printable(attachment))
            }
        }
    }

    #[track_caller]
    fn attach_printable_lazy<A, F>(self, attachment: F) -> Result<T>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::boxed_any(ErrorCategory::Unknown, error)
                    .attach_printable(attachment()))
            }
        }
    }

    #[track_caller]
    fn category(self, category: ErrorCategory) -> Result<T> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::boxed_any(ErrorCategory::Unknown, error).category(category))
            }
        }
    }

    #[track_caller]
    fn change_context<P>(self, context: P) -> Result<T, P>
    where
        P: Context,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::boxed_any(ErrorCategory::Unknown, error).change_context(context))
            }
        }
    }

    #[track_caller]
    fn change_context_lazy<P, F>(self, context: F) -> Result<T, P>
    where
        P: Context,
        F: FnOnce() -> P,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::boxed_any(ErrorCategory::Unknown, error).change_context(context()))
            }
        }
    }

    #[track_caller]
    fn push_context<P>(self, context: P) -> Result<T>
    where
        P: Context,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::boxed_any(ErrorCategory::Unknown, error)
                    .change_context_slient(context))
            }
        }
    }

    #[track_caller]
    fn push_context_lazy<P, F>(self, context: F) -> Result<T>
    where
        P: Context,
        F: FnOnce() -> P,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(EdenError::boxed_any(ErrorCategory::Unknown, error)
                .change_context_slient(context())),
        }
    }
}

impl<T, C: IntoAnyError> AnyResultExtInto for StdResult<T, C> {
    type Ok = T;

    #[track_caller]
    fn anonymize_error_into(self) -> Result<T> {
        match self {
            Ok(n) => Ok(n),
            Err(error) => Err(error.into_eden_any_error()),
        }
    }

    #[track_caller]
    fn attach_into<A>(self, attachment: A) -> Result<T>
    where
        A: Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_any_error().attach(attachment)),
        }
    }

    #[track_caller]
    fn attach_lazy_into<A, F>(self, attachment: F) -> Result<T>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_any_error().attach(attachment())),
        }
    }

    #[track_caller]
    fn attach_printable_into<A>(self, attachment: A) -> Result<T>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_any_error().attach_printable(attachment)),
        }
    }

    #[track_caller]
    fn attach_printable_lazy_into<A, F>(self, attachment: F) -> Result<T>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_any_error().attach_printable(attachment())),
        }
    }

    #[track_caller]
    fn category_into(self, category: ErrorCategory) -> Result<T> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_any_error().category(category)),
        }
    }

    #[track_caller]
    fn change_context_into<P>(self, context: P) -> Result<T, P>
    where
        P: Context,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_any_error().change_context(context)),
        }
    }

    #[track_caller]
    fn change_context_lazy_into<P, F>(self, context: F) -> Result<T, P>
    where
        P: Context,
        F: FnOnce() -> P,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_any_error().change_context(context())),
        }
    }

    #[track_caller]
    fn push_context_into<P>(self, context: P) -> Result<T>
    where
        P: Context,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_any_error().change_context_slient(context)),
        }
    }

    #[track_caller]
    fn push_context_lazy_into<P, F>(self, context: F) -> Result<T>
    where
        P: Context,
        F: FnOnce() -> P,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_any_error().change_context_slient(context())),
        }
    }
}

impl<T, C: Context> ResultExt for StdResult<T, C> {
    type Ok = T;
    type Context = C;

    #[track_caller]
    fn anonymize_error(self) -> Result<T> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(EdenError::context_anonymize(ErrorCategory::Unknown, error)),
        }
    }

    #[track_caller]
    fn attach<A>(self, attachment: A) -> Result<T, C>
    where
        A: Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(EdenError::context(ErrorCategory::Unknown, error).attach(attachment)),
        }
    }

    #[track_caller]
    fn attach_lazy<A, F>(self, attachment: F) -> Result<T, C>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::Unknown, error).attach(attachment()))
            }
        }
    }

    #[track_caller]
    fn attach_printable<A>(self, attachment: A) -> Result<T, C>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::Unknown, error).attach_printable(attachment))
            }
        }
    }

    #[track_caller]
    fn attach_printable_lazy<A, F>(self, attachment: F) -> Result<T, C>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::Unknown, error)
                    .attach_printable(attachment()))
            }
        }
    }

    #[track_caller]
    fn category(self, category: ErrorCategory) -> Result<T, C> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(EdenError::context(ErrorCategory::Unknown, error).category(category)),
        }
    }

    #[track_caller]
    fn change_context<P>(self, context: P) -> Result<T, P>
    where
        P: Context,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::Unknown, error).change_context(context))
            }
        }
    }

    #[track_caller]
    fn change_context_lazy<P, F>(self, context: F) -> Result<T, P>
    where
        P: Context,
        F: FnOnce() -> P,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::Unknown, error).change_context(context()))
            }
        }
    }
}

impl<T, C: Context, E: IntoError<Context = C>> ResultExtInto for StdResult<T, E>
where
    Self: Sized + Send + Sync,
{
    type Ok = T;
    type Context = C;

    #[track_caller]
    fn anonymize_error_into(self) -> Result<T> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_error().anonymize()),
        }
    }

    #[track_caller]
    fn attach_into<A>(self, attachment: A) -> Result<T, C>
    where
        A: Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_error().attach(attachment)),
        }
    }

    #[track_caller]
    fn attach_lazy_into<A, F>(self, attachment: F) -> Result<T, C>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_error().attach(attachment())),
        }
    }

    #[track_caller]
    fn attach_printable_into<A>(self, attachment: A) -> Result<T, C>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_error().attach_printable(attachment)),
        }
    }

    #[track_caller]
    fn attach_printable_lazy_into<A, F>(self, attachment: F) -> Result<T, C>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_error().attach_printable(attachment())),
        }
    }

    #[track_caller]
    fn category_into(self, category: ErrorCategory) -> Result<T, C> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_error().category(category)),
        }
    }

    #[track_caller]
    fn change_context_into<P>(self, context: P) -> Result<T, P>
    where
        P: Context,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_error().change_context(context)),
        }
    }

    #[track_caller]
    fn change_context_lazy_into<P, F>(self, context: F) -> Result<T, P>
    where
        P: Context,
        F: FnOnce() -> P,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.into_eden_error().change_context(context())),
        }
    }
}

impl<T> AnyResultExt for Result<T> {
    type Ok = T;

    #[track_caller]
    fn attach<A>(self, attachment: A) -> Result<T>
    where
        A: Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.attach(attachment)),
        }
    }

    #[track_caller]
    fn attach_lazy<A, F>(self, attachment: F) -> Result<T>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.attach(attachment())),
        }
    }

    #[track_caller]
    fn attach_printable<A>(self, attachment: A) -> Result<T>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.attach_printable(attachment)),
        }
    }

    #[track_caller]
    fn attach_printable_lazy<A, F>(self, attachment: F) -> Result<T>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.attach_printable(attachment())),
        }
    }

    #[track_caller]
    fn category(self, category: ErrorCategory) -> Result<T> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.category(category)),
        }
    }

    #[track_caller]
    fn change_context<P>(self, context: P) -> Result<T, P>
    where
        P: Context,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.change_context(context)),
        }
    }

    #[track_caller]
    fn change_context_lazy<P, F>(self, context: F) -> Result<T, P>
    where
        P: Context,
        F: FnOnce() -> P,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.change_context(context())),
        }
    }

    #[track_caller]
    fn push_context<P>(self, context: P) -> Result<T>
    where
        P: Context,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.change_context_slient(context)),
        }
    }

    #[track_caller]
    fn push_context_lazy<P, F>(self, context: F) -> Result<T>
    where
        P: Context,
        F: FnOnce() -> P,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.change_context_slient(context())),
        }
    }
}

impl<T, C: Context> ResultExt for Result<T, C> {
    type Ok = T;
    type Context = C;

    #[track_caller]
    fn anonymize_error(self) -> Result<T> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.anonymize()),
        }
    }

    #[track_caller]
    fn attach<A>(self, attachment: A) -> Result<T, C>
    where
        A: Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.attach(attachment)),
        }
    }

    #[track_caller]
    fn attach_lazy<A, F>(self, attachment: F) -> Result<T, C>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.attach(attachment())),
        }
    }

    #[track_caller]
    fn attach_printable<A>(self, attachment: A) -> Result<T, C>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.attach_printable(attachment)),
        }
    }

    #[track_caller]
    fn attach_printable_lazy<A, F>(self, attachment: F) -> Result<T, C>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.attach_printable(attachment())),
        }
    }

    #[track_caller]
    fn category(self, category: ErrorCategory) -> Result<T, C> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.category(category)),
        }
    }

    #[track_caller]
    fn change_context<P>(self, context: P) -> Result<T, P>
    where
        P: Context,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.change_context(context)),
        }
    }

    #[track_caller]
    fn change_context_lazy<P, F>(self, context: F) -> Result<T, P>
    where
        P: Context,
        F: FnOnce() -> P,
    {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(error.change_context(context())),
        }
    }
}

/// Implemented shared functions for anonymous [`Error`](EdenError)
/// and typed [`Error`](EdenError).
pub trait ErrorExt {
    #[must_use]
    fn attaches<A>(&self) -> bool
    where
        A: ?Sized + Send + Sync + 'static,
    {
        self.get_attachments::<A>().next().is_some()
    }

    #[must_use]
    fn attach<A>(self, attachment: A) -> Self
    where
        A: Send + Sync + 'static;

    #[must_use]
    fn attach_printable<A>(self, attachment: A) -> Self
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static;

    #[must_use]
    fn category(self, category: ErrorCategory) -> Self;

    #[must_use]
    fn change_context<N>(self, context: N) -> EdenError<N>
    where
        N: Context;

    fn downcast_ref<R>(&self) -> Option<&R>
    where
        R: Context;

    #[must_use]
    fn get_category(&self) -> &ErrorCategory;

    #[must_use]
    fn get_attachments<A>(&self) -> RequestRef<'_, A>
    where
        A: ?Sized + Send + Sync + 'static;
}

/// Implemented shared functions for anonymous [`Error`](EdenError)
/// and typed [`Error`](EdenError).
///
/// Unlike [`ErrorExt`], this trait specializes on anonymous errors
/// because of Rust's trait implementation conflicts.
pub trait ErrorExt2 {
    #[must_use]
    fn attaches<A>(&self) -> bool
    where
        A: ?Sized + Send + Sync + 'static,
    {
        self.get_attachments::<A>().next().is_some()
    }

    #[must_use]
    fn attach<A>(self, attachment: A) -> Self
    where
        A: Send + Sync + 'static;

    #[must_use]
    fn attach_printable<A>(self, attachment: A) -> Self
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static;

    #[must_use]
    fn category(self, category: ErrorCategory) -> Self;

    #[must_use]
    fn change_context<N>(self, context: N) -> EdenError<N>
    where
        N: Context;

    fn downcast_ref<R>(&self) -> Option<&R>
    where
        R: Context;

    #[must_use]
    fn get_category(&self) -> &ErrorCategory;

    #[must_use]
    fn get_attachments<A>(&self) -> RequestRef<'_, A>
    where
        A: ?Sized + Send + Sync + 'static;
}

impl ErrorExt2 for EdenError {
    #[track_caller]
    fn attach<A>(mut self, attachment: A) -> Self
    where
        A: Send + Sync + 'static,
    {
        self.report = self.report.attach(attachment);
        self
    }

    #[track_caller]
    fn attach_printable<A>(mut self, attachment: A) -> Self
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        self.report = self.report.attach_printable(attachment);
        self
    }

    #[track_caller]
    fn category(mut self, category: ErrorCategory) -> Self {
        self.category = category;
        self
    }

    #[track_caller]
    fn change_context<N>(self, context: N) -> EdenError<N>
    where
        N: Context,
    {
        EdenError {
            category: self.category,
            report: self.report.change_context_slient(context),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    fn downcast_ref<R>(&self) -> Option<&R>
    where
        R: Context,
    {
        self.report.downcast_ref::<R>()
    }

    #[track_caller]
    fn get_category(&self) -> &ErrorCategory {
        &self.category
    }

    #[track_caller]
    fn get_attachments<A>(&self) -> RequestRef<'_, A>
    where
        A: ?Sized + Send + Sync + 'static,
    {
        self.report.request_ref::<A>()
    }
}

impl<C: Context> ErrorExt for EdenError<C> {
    #[track_caller]
    fn attach<A>(mut self, attachment: A) -> Self
    where
        A: Send + Sync + 'static,
    {
        self.report = self.report.attach(attachment);
        self
    }

    #[track_caller]
    fn attach_printable<A>(mut self, attachment: A) -> Self
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        self.report = self.report.attach_printable(attachment);
        self
    }

    #[track_caller]
    fn category(mut self, category: ErrorCategory) -> Self {
        self.category = category;
        self
    }

    #[track_caller]
    fn change_context<N>(self, context: N) -> EdenError<N>
    where
        N: Context,
    {
        EdenError {
            category: self.category,
            report: self.report.change_context_slient(context),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    fn downcast_ref<R>(&self) -> Option<&R>
    where
        R: Context,
    {
        self.report.downcast_ref::<R>()
    }

    #[track_caller]
    fn get_category(&self) -> &ErrorCategory {
        &self.category
    }

    #[track_caller]
    fn get_attachments<A>(&self) -> RequestRef<'_, A>
    where
        A: ?Sized + Send + Sync + 'static,
    {
        self.report.request_ref::<A>()
    }
}
