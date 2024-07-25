use dashmap::DashMap;
use eden_utils::error::ResultExt;
use eden_utils::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

mod catch_unwind;
mod config;
mod error;
mod internal;
mod schedule;

use self::error::*;
use self::internal::*;
use crate::Job;

pub use self::config::*;
pub use self::schedule::*;

#[allow(private_interfaces)]
#[derive(Clone)]
pub struct JobRunner<S = BuilderState>(pub(crate) Arc<JobRunnerInner<S>>);

struct JobRunnerInner<S> {
    config: JobRunnerConfig,
    registry: Arc<DashMap<&'static str, JobRegistryMeta<S>>>,
    pool: sqlx::PgPool,
    state: S,
}

impl<S> JobRunner<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub async fn clear_all(&self) -> Result<u64, ClearAllJobsError> {
        internal::clear_all_queued_jobs(self).await
    }

    pub async fn push<J>(&self, job: J) -> Result<(), QueueJobError>
    where
        J: Job<State = S> + Serialize,
    {
        self.queue_job(&job, None)
            .await
            .attach_printable_lazy(|| format!("job.type: {}", J::kind()))
            .attach_printable_lazy(|| format!("job.data: {job:?}"))
    }

    pub async fn schedule<J>(&self, job: J, schedule: Schedule) -> Result<(), QueueJobError>
    where
        J: Job<State = S> + Serialize,
    {
        self.queue_job(&job, Some(schedule))
            .await
            .attach_printable_lazy(|| format!("id: {}", J::kind()))
            .attach_printable_lazy(|| format!("data: {job:?}"))
    }

    pub fn register_job<J>(self) -> Self
    where
        J: Job<State = S> + DeserializeOwned,
    {
        if self.0.registry.contains_key(J::kind()) {
            panic!("Job {:?} is already registered", J::kind());
        }

        let deserializer: DeserializerFn<S> = Box::new(|value| {
            let job: J = serde_json::from_value(value)?;
            Ok(Box::new(job))
        });
        let metadata: JobRegistryMeta<S> = JobRegistryMeta {
            deserializer,
            kind: J::kind(),
            schedule: Box::new(J::schedule),
        };

        self.0.registry.insert(J::kind(), metadata);
        self
    }
}

impl<S> JobRunner<S>
where
    S: Clone + Send + Sync + 'static,
{
    async fn queue_job<J>(&self, job: &J, schedule: Option<Schedule>) -> Result<(), QueueJobError>
    where
        J: Job<State = S> + Serialize,
    {
        // checking if this specified job is registered in the registry
        if !self.0.registry.contains_key(J::kind()) {
            return Err(eden_utils::Error::context(
                eden_utils::ErrorCategory::Unknown,
                QueueJobError,
            ))
            .attach_printable(format!(
                "job {:?} is not registered in the registry",
                J::kind()
            ));
        }

        // make sure that job (with schedule is set to None) has a
        // periodic schedule (retrieved from `J::schedule().is_periodic()`)
        if schedule.is_none() && !J::schedule().is_periodic() {
            return Err(eden_utils::Error::context(
                eden_utils::ErrorCategory::Unknown,
                QueueJobError,
            ))
            .attach_printable(format!(
                "job {:?} is not periodic, consider putting schedule",
                J::kind()
            ));
        }

        internal::insert_into_queue_db(self, job, schedule)
            .await
            .attach_printable("could not queue job into the database")
    }
}

impl<S> std::fmt::Debug for JobRunner<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobRunner")
            .field("config", &self.0.config)
            .field("registered_jobs", &self.0.registry.len())
            .field("state", &std::any::type_name::<S>())
            .finish()
    }
}

impl JobRunner<BuilderState> {
    #[must_use]
    pub const fn builder() -> JobRunnerConfig {
        JobRunnerConfig::new()
    }
}
