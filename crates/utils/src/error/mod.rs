mod any;
mod category;
mod ext;
mod into_impls;

pub use self::category::*;
pub use self::ext::*;
use error_stack::iter::RequestRef;
pub use error_stack::Context;

use self::any::{AnonymizedError, AnyError};
use error_stack::Report;
use std::{fmt, marker::PhantomData};
use tracing_error::SpanTrace;

pub type Result<T, E = AnonymizedError> = std::result::Result<T, Error<E>>;

#[must_use]
pub struct Error<T = AnonymizedError> {
    category: ErrorCategory,
    report: Report,
    trace: SpanTrace,
    _phantom: PhantomData<T>,
}

impl Error {
    #[track_caller]
    pub fn any<T>(category: ErrorCategory, error: T) -> Self
    where
        T: std::error::Error,
    {
        Self {
            category,
            report: AnyError::report(error),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    pub fn context_anonymize(category: ErrorCategory, context: impl Context) -> Self {
        Self {
            category,
            report: Report::new(context).as_any(),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    pub fn report_anonymize(category: ErrorCategory, report: Report<impl Context>) -> Self {
        Self {
            category,
            report: report.as_any(),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    pub fn transform_context<C>(self, context: C) -> Error<C>
    where
        C: Context,
    {
        Error {
            category: self.category,
            report: self.report.change_context_slient(context),
            trace: self.trace,
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    pub fn change_context_slient<C>(mut self, context: C) -> Self
    where
        C: Context,
    {
        self.report = self.report.change_context_slient(context);
        self
    }
}

impl<T: Context> Error<T> {
    #[track_caller]
    pub fn context(category: ErrorCategory, context: T) -> Self {
        Self {
            category,
            report: Report::new(context).as_any(),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    pub fn report(category: ErrorCategory, report: Report<T>) -> Self {
        Self {
            category,
            report: report.as_any(),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    pub fn change_context<C>(self, context: C) -> Error<C>
    where
        C: Context,
    {
        Error {
            category: self.category,
            report: self.report.change_context_slient(context),
            trace: self.trace,
            _phantom: PhantomData,
        }
    }

    #[track_caller]
    pub fn anonymize(self) -> Error {
        Error {
            category: self.category,
            report: self.report,
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Context> Error<T> {
    #[must_use]
    pub fn contains<N>(&self) -> bool
    where
        N: ?Sized + Send + Sync + 'static,
    {
        self.report.request_ref::<T>().next().is_some()
    }

    pub fn get_attached<N>(&self) -> RequestRef<'_, N>
    where
        N: ?Sized + Send + Sync + 'static,
    {
        self.report.request_ref::<N>()
    }
}

impl Error {
    pub fn get_attached_any<N>(&self) -> RequestRef<'_, N>
    where
        N: ?Sized + Send + Sync + 'static,
    {
        self.report.request_ref::<N>()
    }
}

impl Error {
    pub fn init() {
        use error_stack::fmt::{Charset, ColorMode};

        error_stack::Report::set_charset(Charset::Ascii);
        error_stack::Report::set_color_mode(ColorMode::None);
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Error")
            .field("category", &self.category)
            .field("report", &self.report)
            .field("trace", &self.trace)
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Display::fmt(&self.category, f)?;
        writeln!(f, ": {:?}", self.report)?;
        fmt::Display::fmt(&self.trace, f)
    }
}

impl<C> From<Error<C>> for Error
where
    C: Context,
{
    fn from(value: Error<C>) -> Self {
        value.anonymize()
    }
}

// This is for types that do not allow for implement `impl From<Foo> for Error`
pub trait IntoError {
    fn into_eden_error(self) -> Error;
}
