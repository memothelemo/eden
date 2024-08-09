mod run_context;
mod trigger;

pub use self::run_context::TaskRunContext;
pub use self::trigger::*;

pub use eden_tasks_schema::types::{TaskPriority, TaskStatus};

use async_trait::async_trait;
use chrono::TimeDelta;
use eden_utils::Result;
use std::fmt::Debug;

#[derive(Debug)]
pub enum TaskResult {
    /// The task has completed its task.
    Completed,
    /// The task has encountered a rejected error and should not
    /// be tried again.
    ///
    /// If the task running encountered this fatal error, it will
    /// not attempt to any backoffs.
    Reject(eden_utils::Error),
    /// The task will try to run again in the future within
    /// the given duration. This attempt will be counted to the
    /// total number of consecutive retries.
    RetryIn(TimeDelta),
}

#[async_trait]
pub trait Task: Debug + Send + Sync + 'static {
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
    fn kind() -> &'static str
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

    /// The conditions that will trigger the task to run.
    ///
    /// If the trigger is set other than [`TaskTrigger::None`], this task is
    /// assumed to be a recurring task and should be ran periodically.
    ///
    /// Recurring tasks are not used to schedule at a different time in demand
    /// unless it fails to perform the operation and will run at a later time.
    ///
    /// It defaults to [`TaskTrigger::None`]
    fn trigger() -> TaskTrigger
    where
        Self: Sized,
    {
        TaskTrigger::None
    }

    /// It determines whether a task is temporary and lasts the entire
    /// program lifetime.
    ///
    /// If a task is temporary and queued to the database, it will be deleted
    /// upon startup.
    ///
    /// This is useful for tasks like setting up global commands to Discord.
    /// Since this task may fail, it will run again if it fails under the condition
    /// that the program should not be terminated until it is in due.
    fn temporary() -> bool
    where
        Self: Sized,
    {
        false
    }

    /// The delay before a task is processed again after an error.
    ///
    /// It starts with 1 minute, then 2 minutes and so on.
    fn backoff(&self, retries: u16) -> TimeDelta {
        super::backoff::exponential(TimeDelta::minutes(1), 2, retries)
    }

    /// The maximum amount of retries before a task is marked as failed.
    fn max_retries(&self) -> u16 {
        5
    }

    /// The maximum amount of time for the task to be waited before
    /// marking it as failed.
    ///
    /// It defaults to 10 minutes.
    fn timeout(&self) -> TimeDelta {
        TimeDelta::minutes(10)
    }

    /// This function will attempt to perform an operation for a task.
    async fn perform(&self, ctx: &TaskRunContext, state: Self::State) -> Result<TaskResult>;
}

#[cfg(test)]
mod tests {
    use super::Task;

    use static_assertions::assert_obj_safe;
    use std::sync::Arc;

    assert_obj_safe!(Task<State = Arc<()>>);
}
