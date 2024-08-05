use thiserror::Error;

pub mod tags;

#[derive(Debug, Error)]
#[error("could not start queue worker")]
pub struct WorkerStartError;

#[derive(Debug, Error)]
#[error("task failed")]
pub struct TaskError;

#[derive(Debug, Error)]
#[error("could not schedule task")]
pub struct ScheduleTaskError;

#[derive(Debug, Error)]
#[error("could not perform task")]
pub(crate) struct PerformTaskError;

#[derive(Debug, Error)]
#[error("could not delete task")]
pub(crate) struct DeleteTaskError;

#[derive(Debug, Error)]
#[error("could not clear all task(s)")]
pub(crate) struct ClearAllTasksError;

#[derive(Debug, Error)]
#[error("could not clear temporary task(s)")]
pub(crate) struct ClearTemporaryTasksError;

#[derive(Debug, Error)]
#[error("could not update recurring task blacklist")]
pub(crate) struct UpdateTaskBlacklistError;
