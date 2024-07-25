use eden_utils::error::{AnyResultExt, ResultExt};
use futures::{future::BoxFuture, Future, FutureExt};
use std::task::Poll;

use super::RunJobError;
use crate::JobResult;

#[must_use = "Futures are lazy, call `.await` to perform a job"]
pub struct CatchUnwindJobFuture<'a> {
    future: BoxFuture<'a, eden_utils::Result<JobResult>>,
}

impl<'a> CatchUnwindJobFuture<'a> {
    pub fn new(future: BoxFuture<'a, eden_utils::Result<JobResult>>) -> Self {
        Self { future }
    }
}

impl<'a> Future for CatchUnwindJobFuture<'a> {
    type Output = eden_utils::Result<JobResult, RunJobError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let future = &mut self.future;

        let result = match catch_unwind(move || future.poll_unpin(cx)) {
            Ok(Poll::Pending) => return Poll::Pending,
            Ok(Poll::Ready(value)) => value.transform_context(RunJobError),
            Err(error) => Err(error),
        };

        Poll::Ready(result)
    }
}

#[track_caller]
fn catch_unwind<F: FnOnce() -> R, R>(f: F) -> eden_utils::Result<R, RunJobError> {
    #[derive(Debug)]
    pub struct JobPanicked(String);

    impl std::fmt::Display for JobPanicked {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl eden_utils::error::Context for JobPanicked {}

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(res) => Ok(res),
        Err(cause) => {
            let cause = cause
                .downcast_ref::<&'static str>()
                .map(|v| v.to_string())
                .or_else(|| cause.downcast_ref::<String>().map(|v| v.to_string()))
                .unwrap_or_else(|| "unknown".into());

            Err(JobPanicked(cause))
        }
    }
    .change_context(RunJobError)
    .attach_printable("job panicked while the job ran")
}
