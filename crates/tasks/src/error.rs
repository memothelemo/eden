use thiserror::Error;

#[derive(Debug, Error)]
#[error("unable to start queue")]
pub struct StartQueueError;

#[derive(Debug, Error)]
#[error("could not clear all task(s)")]
pub struct ClearAllTasksError;

#[derive(Debug, Error)]
#[error("could not delete task")]
pub struct DeleteTaskError;

#[derive(Debug, Error)]
#[error("could not perform task")]
pub struct PerformTaskError;

#[derive(Debug, Error)]
#[error("could not schedule task")]
pub struct ScheduleTaskError;

#[derive(Debug, Error)]
#[error("invalid cron expression")]
pub struct InvalidCronExpr;
