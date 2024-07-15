use error_stack::Context;
use std::fmt;
use std::marker::PhantomData;

use crate::error::{Error as EdenError, ErrorCategory, Result};

pub trait ResultExt {
    type Context: Context;
    type Ok;

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

    fn change_context<C>(self, context: C) -> Result<Self::Ok, C>
    where
        C: Context;

    fn change_context_lazy<C, F>(self, context: F) -> Result<Self::Ok, C>
    where
        C: Context,
        F: FnOnce() -> C;
}

pub trait AnyResultExt {
    type Ok;

    fn anonymize_error(self) -> Result<Self::Ok>;

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

    fn change_context_slient<C>(self, context: C) -> Result<Self::Ok>
    where
        C: Context;

    fn change_context_slient_lazy<C, F>(self, context: F) -> Result<Self::Ok>
    where
        C: Context,
        F: FnOnce() -> C;

    fn transform_context<C>(self, context: C) -> Result<Self::Ok, C>
    where
        C: Context;

    fn transform_context_lazy<C, F>(self, context: F) -> Result<Self::Ok, C>
    where
        C: Context,
        F: FnOnce() -> C;
}

impl<T, C> ResultExt for core::result::Result<T, C>
where
    C: Context,
{
    type Context = C;
    type Ok = T;

    #[track_caller]
    fn anonymize_error(self) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(EdenError::context_anonymize(
                ErrorCategory::default(),
                error,
            )),
        }
    }

    #[track_caller]
    fn attach<A>(self, attachment: A) -> Result<T, C>
    where
        A: Send + Sync + 'static,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::default(), error).attach(attachment))
            }
        }
    }

    #[track_caller]
    fn attach_lazy<A, F>(self, attachment: F) -> Result<T, C>
    where
        A: Send + Sync + 'static,
        F: FnOnce() -> A,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::default(), error).attach(attachment()))
            }
        }
    }

    #[track_caller]
    fn attach_printable<A>(self, attachment: A) -> Result<T, C>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::default(), error)
                    .attach_printable(attachment))
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
            Ok(value) => Ok(value),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::default(), error)
                    .attach_printable(attachment()))
            }
        }
    }

    #[track_caller]
    fn category(self, category: ErrorCategory) -> Result<T, C> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(EdenError::context(category, error)),
        }
    }

    #[track_caller]
    fn change_context<C2>(self, context: C2) -> Result<T, C2>
    where
        C2: Context,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::default(), error).change_context(context))
            }
        }
    }

    #[track_caller]
    fn change_context_lazy<C2, F>(self, context: F) -> Result<T, C2>
    where
        C2: Context,
        F: FnOnce() -> C2,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                Err(EdenError::context(ErrorCategory::default(), error).change_context(context()))
            }
        }
    }
}

impl<T, C> ResultExt for Result<T, C>
where
    C: Context,
{
    type Context = C;
    type Ok = T;

    #[track_caller]
    fn anonymize_error(self) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.anonymize()),
        }
    }

    #[track_caller]
    fn attach<A>(self, attachment: A) -> Result<T, C>
    where
        A: Send + Sync + 'static,
    {
        match self {
            Ok(value) => Ok(value),
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
            Ok(value) => Ok(value),
            Err(error) => Err(error.attach(attachment())),
        }
    }

    #[track_caller]
    fn attach_printable<A>(self, attachment: A) -> Result<T, C>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            Ok(value) => Ok(value),
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
            Ok(value) => Ok(value),
            Err(error) => Err(error.attach_printable(attachment())),
        }
    }

    #[track_caller]
    fn category(self, category: ErrorCategory) -> Result<T, C> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.category(category)),
        }
    }

    #[track_caller]
    fn change_context<C2>(self, context: C2) -> Result<T, C2>
    where
        C2: Context,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.change_context(context)),
        }
    }

    #[track_caller]
    fn change_context_lazy<C2, F>(self, context: F) -> Result<T, C2>
    where
        C2: Context,
        F: FnOnce() -> C2,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.change_context(context())),
        }
    }
}

impl<T> AnyResultExt for Result<T> {
    type Ok = T;

    #[track_caller]
    fn anonymize_error(self) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error),
        }
    }

    #[track_caller]
    fn attach<A>(self, attachment: A) -> Result<T>
    where
        A: Send + Sync + 'static,
    {
        match self {
            Ok(value) => Ok(value),
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
            Ok(value) => Ok(value),
            Err(error) => Err(error.attach(attachment())),
        }
    }

    #[track_caller]
    fn attach_printable<A>(self, attachment: A) -> Result<T>
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        match self {
            Ok(value) => Ok(value),
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
            Ok(value) => Ok(value),
            Err(error) => Err(error.attach_printable(attachment())),
        }
    }

    #[track_caller]
    fn category(self, category: ErrorCategory) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.category(category)),
        }
    }

    #[track_caller]
    fn change_context_slient<C>(self, context: C) -> Result<T>
    where
        C: Context,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.change_context_slient(context)),
        }
    }

    #[track_caller]
    fn change_context_slient_lazy<C, F>(self, context: F) -> Result<T>
    where
        C: Context,
        F: FnOnce() -> C,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.change_context_slient(context())),
        }
    }

    #[track_caller]
    fn transform_context<C>(self, context: C) -> Result<T, C>
    where
        C: Context,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.change_context(context)),
        }
    }

    #[track_caller]
    fn transform_context_lazy<C, F>(self, context: F) -> Result<T, C>
    where
        C: Context,
        F: FnOnce() -> C,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(error.change_context(context())),
        }
    }
}

pub trait ErrorExt {
    fn get_category(&self) -> &ErrorCategory;
    #[must_use]
    fn attach<A>(self, attachment: A) -> Self
    where
        A: Send + Sync + 'static;

    #[must_use]
    fn attach_printable<A>(self, attachment: A) -> Self
    where
        A: fmt::Display + fmt::Debug + Send + Sync + 'static;

    fn change_context<T>(self, context: T) -> EdenError<T>
    where
        T: Context;

    #[must_use]
    fn category(self, category: ErrorCategory) -> Self;

    fn downcast_ref<F>(&self) -> Option<&F>
    where
        F: Context;
}

macro_rules! impl_error_ext {
    () => {
        #[track_caller]
        fn get_category(&self) -> &ErrorCategory {
            &self.category
        }

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
        fn change_context<N>(self, context: N) -> EdenError<N>
        where
            N: Context,
        {
            EdenError {
                category: self.category,
                report: self.report.change_context_slient(context),
                trace: self.trace,
                _phantom: PhantomData,
            }
        }

        #[track_caller]
        fn category(self, category: ErrorCategory) -> Self {
            Self {
                category,
                report: self.report,
                trace: self.trace,
                _phantom: PhantomData,
            }
        }

        #[track_caller]
        fn downcast_ref<F: Context>(&self) -> Option<&F> {
            self.report.downcast_ref()
        }
    };
}

impl ErrorExt for EdenError {
    impl_error_ext!();
}

impl<T: Context> ErrorExt for EdenError<T> {
    impl_error_ext!();
}
