use eden_utils::error::{AnyResultExt, ResultExt};
use futures::{future::BoxFuture, Future, FutureExt};
use std::task::Poll;

use super::PerformTaskError;
use crate::TaskResult;

#[must_use = "Futures are lazy, call `.await` to perform a task"]
pub struct CatchUnwindTaskFuture<'a> {
    future: BoxFuture<'a, eden_utils::Result<TaskResult>>,
}

impl<'a> CatchUnwindTaskFuture<'a> {
    pub fn new(future: BoxFuture<'a, eden_utils::Result<TaskResult>>) -> Self {
        Self { future }
    }
}

impl<'a> Future for CatchUnwindTaskFuture<'a> {
    type Output = eden_utils::Result<TaskResult, PerformTaskError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let future = &mut self.future;

        let result = match catch_unwind(move || future.poll_unpin(cx)) {
            Ok(Poll::Pending) => return Poll::Pending,
            Ok(Poll::Ready(value)) => value.transform_context(PerformTaskError),
            Err(error) => Err(error),
        };

        Poll::Ready(result)
    }
}

#[track_caller]
fn catch_unwind<F: FnOnce() -> R, R>(f: F) -> eden_utils::Result<R, PerformTaskError> {
    #[derive(Debug)]
    pub struct TaskPanicked(String);

    impl std::fmt::Display for TaskPanicked {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl eden_utils::error::Context for TaskPanicked {}

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(res) => Ok(res),
        Err(cause) => {
            let cause = cause
                .downcast_ref::<&'static str>()
                .map(std::string::ToString::to_string)
                .or_else(|| cause.downcast_ref::<String>().map(String::to_string))
                .unwrap_or_else(|| "unknown".into());

            Err(TaskPanicked(cause))
        }
    }
    .change_context(PerformTaskError)
    .attach_printable("task panicked while the task ran")
    .attach(super::error_tags::TaskPanicked)
}
