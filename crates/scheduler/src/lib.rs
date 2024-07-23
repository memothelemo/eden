pub mod backoff;
pub mod job;
pub mod runner;

pub use self::job::{Job, JobSchedule, JobStatus};

use chrono::TimeDelta;
use futures::future::BoxFuture;
use runner::{JobRunner, Schedule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CleanupUsers;

impl Job for CleanupUsers {
    type State = State;

    fn id() -> &'static str
    where
        Self: Sized,
    {
        "cleanup-users"
    }

    fn schedule() -> JobSchedule
    where
        Self: Sized,
    {
        JobSchedule::interval(TimeDelta::seconds(5))
    }

    fn run(&self, _state: Self::State) -> BoxFuture<'_, eden_utils::Result<JobStatus>> {
        Box::pin(async { Ok(JobStatus::Completed) })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State;

fn that(pool: sqlx::PgPool) {
    let executor = JobRunner::new(pool, State)
        .register_job::<CleanupUsers>()
        .register_job::<CleanupUsers>();

    executor.schedule(CleanupUsers, Schedule::now()).unwrap();
}
