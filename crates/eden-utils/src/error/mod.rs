use error_stack::iter::RequestRef;
use error_stack::Report;
use std::fmt;
use std::marker::PhantomData;
use tracing_error::SpanTrace;

pub(crate) mod any;
mod category;
mod into_error;
mod into_result;

pub mod exts;
pub mod prelude;
pub mod tags;

pub use self::category::*;
pub use ::error_stack::Context;

pub type Result<T, E = self::any::AnonymizedError> = std::result::Result<T, Error<E>>;

#[must_use]
pub struct Error<T = self::any::AnonymizedError> {
    pub(crate) category: ErrorCategory,
    pub(crate) report: Report,
    trace: SpanTrace,
    _phantom: PhantomData<T>,
}

impl Error {
    /// Creates an [`Error`] with [`ErrorCategory::Unknown`] as its category
    /// from objects implemented with [`std::error::Error`].
    #[track_caller]
    pub fn unknown<T>(error: T) -> Self
    where
        T: std::error::Error + 'static,
    {
        Self {
            category: ErrorCategory::Unknown,
            report: self::any::report_from_any_error(&error),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    /// Creates an [`Error`] with [`ErrorCategory::Unknown`] as its category
    /// from any boxed errors.
    #[track_caller]
    pub fn boxed_any(
        category: ErrorCategory,
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
    ) -> Self {
        Self {
            category,
            report: self::any::report_from_boxed_error(&error),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    /// Creates an [`Error`] from objects implemented with [`std::error::Error`].
    #[track_caller]
    pub fn any<T>(category: ErrorCategory, error: T) -> Self
    where
        T: std::error::Error + 'static,
    {
        Self {
            category,
            report: self::any::report_from_any_error(&error),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    /// Creates an [`Error`] but the context argument will conceal its
    /// type and become anonymous.
    #[track_caller]
    pub fn context_anonymize(category: ErrorCategory, context: impl Context) -> Self {
        Self {
            category,
            report: Report::new(context).as_any(),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    /// Creates an [`Error`] with [`Report`] type. Any report argument
    /// with [`Report`] type is accepted as long as it has a type
    /// implemented with [`error_stack::Context`].
    #[track_caller]
    pub fn report_anonymize(category: ErrorCategory, report: Report<impl Context>) -> Self {
        Self {
            category,
            report: report.as_any(),
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    #[must_use]
    pub fn anonymized_report(category: ErrorCategory, report: Report) -> Self {
        Self {
            category,
            report,
            trace: SpanTrace::capture(),
            _phantom: PhantomData,
        }
    }

    /// Add a new [`Context`] object to the top of the frame stack
    /// without affecting the type of the [`Error`] object.
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

    pub fn downcast_ref<N>(&self) -> Option<&N>
    where
        N: Send + Sync + 'static,
    {
        self.report.downcast_ref::<N>()
    }

    pub fn get_attached<N>(&self) -> RequestRef<'_, N>
    where
        N: ?Sized + Send + Sync + 'static,
    {
        self.report.request_ref::<N>()
    }
}

impl Error {
    pub fn downcast_ref_any<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        self.report.downcast_ref::<T>()
    }

    pub fn get_attached_any<N>(&self) -> RequestRef<'_, N>
    where
        N: ?Sized + Send + Sync + 'static,
    {
        self.report.request_ref::<N>()
    }
}

impl Error {
    /// Wrapper of [`Report::install_debug_hook`].
    pub fn install_hook<T: Send + Sync + 'static>(
        hook: impl Fn(&T, &mut error_stack::fmt::HookContext<T>) + Send + Sync + 'static,
    ) {
        error_stack::Report::install_debug_hook::<T>(hook);
    }

    /// Installs hooks from all errors and tags in [`eden_utils`](crate)
    /// and sets up preferences from [`error_stack`] tailored for Eden.
    pub fn init() {
        use self::tags::Suggestion;
        use crate::sql::tags::{DatabaseErrorType, PostgresErrorInfo};
        use crate::twilight::tags::DiscordHttpErrorInfo;

        use error_stack::fmt::{Charset, ColorMode};

        Report::set_charset(Charset::Ascii);
        Report::set_color_mode(ColorMode::None);

        Suggestion::install_hook();
        DatabaseErrorType::install_hook();
        PostgresErrorInfo::install_hook();
        DiscordHttpErrorInfo::install_hook();
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

// Anonymize values automatically from functions where it returns
// a Result with an anonymous error.
impl<C> From<Error<C>> for Error
where
    C: Context,
{
    #[track_caller]
    fn from(value: Error<C>) -> Self {
        value.anonymize()
    }
}

impl<C: self::into_error::IntoError> From<C> for Error<C::Context> {
    #[track_caller]
    fn from(value: C) -> Self {
        value.into_eden_error()
    }
}
