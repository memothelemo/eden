use chrono::{DateTime, TimeDelta, Utc};
use eden_utils::Result;
use futures::future::BoxFuture;
use std::borrow::Cow;
use std::fmt::Debug;
use uuid::Uuid;

mod schedule;
mod scheduled;

pub use self::schedule::*;
pub use self::scheduled::Scheduled;
pub use eden_db::schema::{TaskPriority, TaskStatus};

/// It contains data of what, where and when this task is being performed at.
#[derive(Debug, PartialEq, Eq)]
pub struct TaskPerformInfo {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub failed_attempts: i32,
    pub last_retry: Option<DateTime<Utc>>,
    pub is_retrying: bool,
}

// We need this trait depend on Deserialize & Serialize so that we can
// actually process it into the database and do other things later on.
pub trait Task: Debug + Send + Sync + 'static {
    // It must be cloned, preferably wrapped with std::sync::Arc type.
    type State: Clone + Send + Sync + 'static;

    /// A **unique** type of the task. It is used to differentiate different types
    /// of tasks in the database and deserialize and serialize the data given
    /// per task.
    ///
    /// <b>
    /// Make sure you configure the unique type of the task CORRECT AND FINAL
    /// as any changes to the task identifier will not reflected to the database
    /// (unless manually edited) and might get an unexpected error in logging.
    /// </b>
    fn task_type() -> &'static str
    where
        Self: Sized;

    /// The priority of which task should go first if the deadline is in
    /// the similar range with other tasks where [`TaskPriority::High`] has
    /// the highest priority and [`TaskPriority::Low`] being the lowest.
    ///
    /// It defaults to [`TaskPriority::Medium`].
    fn priority() -> TaskPriority
    where
        Self: Sized,
    {
        TaskPriority::Medium
    }

    /// The periodic schedule of a task of when it should be ran.
    ///
    /// It defaults to [`TaskSchedule::Once`].
    fn schedule() -> TaskSchedule
    where
        Self: Sized,
    {
        TaskSchedule::Once
    }

    /// The delay before a task is processed again after an error.
    ///
    /// It starts with 1 minute, then 2 minutes and so on.
    fn backoff(&self, retries: u16) -> TimeDelta {
        super::backoff::exponential(TimeDelta::minutes(1), 2, retries)
    }

    /// The maximum amount of retries before a task is marked
    /// as failed.
    fn max_retries(&self) -> u16 {
        5
    }

    /// The maximum amount of time for the task will be waited before
    /// marking it as failed.
    ///
    /// It defaults to 10 minutes.
    fn timeout(&self) -> TimeDelta {
        TimeDelta::minutes(10)
    }

    /// This function will attempt to perform an operation from task.
    ///
    /// Its return type, [`TaskResult`] determines whether the task needs to be
    /// retried again or ignored/retried again in a very later time after it
    /// receives a successful status.
    fn perform(
        &self,
        info: &TaskPerformInfo,
        state: Self::State,
    ) -> BoxFuture<'_, Result<TaskResult>>;
}

#[derive(Debug)]
pub enum TaskResult {
    /// The task has completed its task.
    Completed,
    /// The task has encountered a fatal error and should not
    /// be tried again.
    ///
    /// If the task running encountered this fatal error, it will
    /// not attempt to any backoffs.
    Fail(eden_utils::Error),
    /// The task will try to run again in the future within
    /// the given duration. This attempt will be counted to the
    /// total number of consecutive retries.
    RetryIn(TimeDelta),
}

#[cfg(test)]
mod tests {
    use super::Task;

    use static_assertions::assert_obj_safe;
    use std::sync::Arc;

    assert_obj_safe!(Task<State = Arc<()>>);
}
