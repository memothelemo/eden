use error_stack::Report;
use std::error::Error as StdError;

// Rust won't let me use 'impl ErrorExt for EdenError<()>' because the tuple type
// may be added by Rust devs in later versions. (obviously it won't)
//
/// Tag used for [`Error`] to indicate that this is an anonymous error
pub struct AnonymizedError;

/// error-stack friendly version of any errors.
struct AnyError(String);

#[allow(clippy::unwrap_used)]
#[track_caller]
pub fn report_from_boxed_error(error: &Box<dyn StdError + Send + Sync + 'static>) -> Report {
    let mut report = None;

    // Try #1, try to base from the error's source then we can
    // manually pushing it to the Report.
    if let Some(source) = error.source() {
        report = Some(report_from_any_error(source));
    }

    match report {
        Some(n) => n.change_context_slient(AnyError(error.to_string())),
        None => Report::new(AnyError(error.to_string())).as_any(),
    }
}

#[allow(clippy::unwrap_used)]
#[track_caller]
pub fn report_from_any_error(error: &(dyn StdError + 'static)) -> Report {
    let error: &dyn StdError = error;

    let mut report = None;
    let mut sources = Sources {
        current: Some(error),
    }
    .collect::<Vec<_>>();
    sources.reverse();

    for source in sources {
        let context = AnyError(source.to_string());
        if report.is_none() {
            report = Some(Report::new(context));
        } else {
            report = match report {
                Some(v) => Some(v.change_context(context)),
                None => None,
            };
        }
    }

    report.unwrap().as_any()
}

impl std::fmt::Debug for AnyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl std::fmt::Display for AnyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl error_stack::Context for AnyError {}

struct Sources<'a> {
    current: Option<&'a (dyn StdError + 'static)>,
}

impl<'a> Iterator for Sources<'a> {
    type Item = &'a (dyn StdError + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current = self.current.and_then(StdError::source);
        current
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.current.is_some() {
            (1, None)
        } else {
            (0, Some(0))
        }
    }
}

#[allow(unused, clippy::let_underscore_must_use)]
#[cfg(test)]
mod tests {
    use super::{report_from_any_error, report_from_boxed_error};
    use std::error::Error as StdError;
    use std::sync::LazyLock;
    use thiserror::Error;

    #[derive(Debug, Error)]
    #[error("test error")]
    struct TestError;

    // Should accept values from:
    // - Any structs implemented with StdError
    // - Box<dyn StdError + Sync + Send + 'static>
    const SHOULD_ACCEPT_VALUES: LazyLock<()> = LazyLock::new(|| {
        report_from_any_error(&TestError);
        report_from_any_error(&Box::new(TestError));

        let any: Box<dyn StdError + Send + Sync + 'static> = Box::new(TestError);
        report_from_boxed_error(&any);
    });
}
