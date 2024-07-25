use chrono::Utc;
use dashmap::DashMap;
use eden_db::forms::UpdateJobForm;
use eden_db::schema::Job as JobSchema;
use eden_db::schema::JobStatus;
use eden_utils::error::AnyResultExt;
use eden_utils::error::ResultExt;
use eden_utils::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

mod catch_unwind;
mod config;
mod error;
mod internal;
mod schedule;

use self::internal::*;
use crate::Job;
use crate::JobResult;

pub use self::config::*;
pub use self::error::*;
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

    pub async fn queue_failed_jobs(&self) -> Result<(), QueueFailedJobsError> {
        let mut conn = self
            .transaction()
            .await
            .transform_context(QueueFailedJobsError)
            .attach_printable("could not start database transaction")?;

        let mut queue = JobSchema::get_all().status(JobStatus::Failed).build();
        let now = Utc::now();

        while let Some(jobs) = queue
            .next(&mut conn)
            .await
            .change_context(QueueFailedJobsError)
            .attach_printable("could not pull failed jobs")?
        {
            for job in jobs {
                let form = UpdateJobForm::builder()
                    .status(Some(JobStatus::Failed))
                    .build();

                internal::provide_job_data_if_error::<_, _, S>(
                    &job,
                    None,
                    now,
                    None,
                    JobSchema::update(&mut conn, job.id, form)
                        .await
                        .change_context(QueueFailedJobsError)
                        .attach_printable("could not update status of a failed job"),
                )?;
            }
        }

        conn.commit()
            .await
            .change_context(QueueFailedJobsError)
            .attach_printable("could not commit database transaction")?;

        Ok(())
    }

    pub async fn process_routine_jobs(&self) -> Result<(), ProcessRoutineJobsError> {
        todo!()
    }

    pub async fn process_queued_jobs(&self) -> Result<(), ProcessQueuedJobsError> {
        let mut conn = self
            .transaction()
            .await
            .transform_context(ProcessQueuedJobsError)
            .attach_printable("could not start database transaction")?;

        // There are 32 bits to work for this value.
        let max_failed_attempts = self.0.config.max_failed_attempts as i64;
        let now = Utc::now();

        let mut queue = JobSchema::pull_all_pending(max_failed_attempts, Some(now)).size(50);
        while let Some(jobs) = queue
            .next(&mut conn)
            .await
            .change_context(ProcessQueuedJobsError)
            .attach_printable("could not pull jobs")?
        {
            println!("pulled {} jobs", jobs.len());

            for job in jobs {
                let result = internal::provide_job_data_if_error::<_, _, S>(
                    &job,
                    None,
                    now,
                    None,
                    self.try_run_unknown_job(&mut conn, &job).await,
                )
                .attach_printable("could not run job");

                if let Err(error) = result {
                    eprintln!("Could not run job: {}", error.anonymize());
                }
            }
        }

        conn.commit()
            .await
            .change_context(ProcessQueuedJobsError)
            .attach_printable("could not commit database transaction")?;

        Ok(())
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
    async fn transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
        self.0.pool.begin().await.anonymize_error()
    }

    async fn try_run_unknown_job(
        &self,
        conn: &mut sqlx::PgConnection,
        schema: &JobSchema,
    ) -> Result<(), RunJobError> {
        // Search for that type of job from the registry
        let kind = schema.data.kind.as_str();
        let Some(registry_meta) = self.0.registry.get(kind) else {
            return Err(eden_utils::Error::context(
                eden_utils::ErrorCategory::Unknown,
                RunJobError,
            ))
            .attach_printable(format!("unknown job {kind:?} (not registered in registry)"));
        };

        let deserializer = &*registry_meta.deserializer;
        let job = deserializer(schema.data.data.clone())
            .map_err(|e| eden_utils::Error::any(eden_utils::ErrorCategory::Unknown, e))
            .transform_context(RunJobError)
            .attach_printable_lazy(|| {
                format!("could not deserialize job {:?}", registry_meta.kind)
            })?;

        println!(
            "running job {} with type {:?}; data = {job:?}",
            schema.id, registry_meta.kind
        );

        match internal::run_job(self, &*job, &registry_meta).await {
            Ok(new_status) => match new_status {
                JobResult::Completed => {
                    println!("completed");
                }
                JobResult::Fail(_) => {
                    println!("failed");
                }
                JobResult::RetryIn(_) => {
                    println!("retry");
                }
            },
            Err(error) => {
                JobSchema::fail(conn, schema.id)
                    .await
                    .change_context(RunJobError)
                    .attach_printable("could not fail job")?;

                return Err(error);
            }
        }

        Ok(())
    }

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
