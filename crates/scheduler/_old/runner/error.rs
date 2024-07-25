use thiserror::Error;

#[derive(Debug, Error)]
#[error("could not serialize task data")]
pub struct SerializeTaskError;

#[derive(Debug, Error)]
#[error("could not process queue task(s)")]
pub struct ProcessQueuedTasksError;

#[derive(Debug, Error)]
#[error("could not process routine task(s)")]
pub struct ProcessRoutineTasksError;

#[derive(Debug, Error)]
#[error("could not queue failed task(s)")]
pub struct QueueFailedTasksError;

#[derive(Debug, Error)]
#[error("could not queue task")]
pub struct QueueTaskError;

#[derive(Debug, Error)]
#[error("could not clear all tasks")]
pub struct ClearAllTasksError;

#[derive(Debug, Error)]
#[error("could not run task")]
pub struct RunTaskError;

#[derive(Debug, Error)]
#[error("task got timed out")]
pub struct TaskTimedOut;
