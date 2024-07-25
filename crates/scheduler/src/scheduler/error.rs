use thiserror::Error;

#[derive(Debug, Error)]
#[error("could not clear all queue task(s)")]
pub struct ClearAllTasksError;

#[derive(Debug, Error)]
#[error("could not queue task")]
pub struct QueueTaskError;

#[derive(Debug, Error)]
#[error("could not perform task")]
pub struct PerformTaskError;

#[derive(Debug, Error)]
#[error("could not serialize task data")]
pub struct SerializeTaskError;
