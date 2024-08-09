use error_stack::{iter::RequestRef, Context};
use std::fmt;
use std::marker::PhantomData;
use tracing_error::SpanTrace;

use super::{Error as EdenError, ErrorCategory, Result};

pub use super::into_error::*;
pub use super::into_result::*;

/// Implements shared functions for [`Result`](std::result::Result) where
/// its error type is a typed [`Error`](Eden).
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

/// Implements shared functions for [`Result`](std::result::Result) where
/// its error type is a typed [`Error`](Eden).
pub trait ResultExt2 {
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

/// Implements shared functions for [`Result`](std::result::Result) where
/// its error type is an anonymized [`Error`](Eden).
pub trait AnonymizedResultExt {
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

    fn change_context_lazy<P, F>(self, context: F) -> Result<Self::Ok, P>
    where
        P: Context,
        F: FnOnce() -> P;

    #[track_caller]
    fn push_context<P>(self, context: P) -> Result<Self::Ok>
    where
        P: Context;

    #[track_caller]
    fn push_context_lazy<P, F>(self, context: F) -> Result<Self::Ok>
    where
        P: Context,
        F: FnOnce() -> P;
}

impl<T, C> ResultExt for Result<T, C>
where
    C: Context,
{
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

impl<T> AnonymizedResultExt for Result<T> {
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

/// Implements shared functions for anonymous [`Error`](EdenError)
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
pub trait AnyErrorExt {
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

impl AnyErrorExt for EdenError {
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
