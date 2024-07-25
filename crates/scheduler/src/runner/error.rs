use thiserror::Error;

#[derive(Debug, Error)]
#[error("could not serialize job data")]
pub struct SerializeJobError;

#[derive(Debug, Error)]
#[error("could not process queue job(s)")]
pub struct ProcessQueuedJobsError;

#[derive(Debug, Error)]
#[error("could not process routine job(s)")]
pub struct ProcessRoutineJobsError;

#[derive(Debug, Error)]
#[error("could not queue failed job(s)")]
pub struct QueueFailedJobsError;

#[derive(Debug, Error)]
#[error("could not queue job")]
pub struct QueueJobError;

#[derive(Debug, Error)]
#[error("could not clear all jobs")]
pub struct ClearAllJobsError;

#[derive(Debug, Error)]
#[error("could not run job")]
pub struct RunJobError;

#[derive(Debug, Error)]
#[error("job got timed out")]
pub struct JobTimedOut;
