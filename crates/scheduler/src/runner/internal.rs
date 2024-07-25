use crate::runner::catch_unwind::CatchUnwindJobFuture;
use crate::runner::JobTimedOut;
use crate::{Job, JobRunner, JobSchedule, JobStatus, Schedule};

use chrono::{DateTime, Utc};
use eden_db::forms::InsertJobForm;
use eden_db::schema::{Job as JobSchema, JobRawData};
use eden_utils::error::ResultExt;
use eden_utils::Result;
use futures::FutureExt;
use serde::Serialize;
use serde_json::Value as Json;

use super::{ClearAllJobsError, QueueJobError, RunJobError, SerializeJobError};

pub struct BuilderState;

pub struct JobRegistryMeta<S> {
    pub(crate) deserializer: DeserializerFn<S>,
    pub(crate) kind: &'static str,
    pub(crate) schedule: ScheduleFn,
}

pub type ScheduleFn = Box<dyn Fn() -> JobSchedule>;
pub type DeserializerFn<State> = Box<
    dyn Fn(
            Json,
        ) -> std::result::Result<
            Box<dyn Job<State = State>>,
            Box<dyn std::error::Error + Send + Sync>,
        > + Send
        + Sync
        + 'static,
>;

pub fn provide_job_data_if_error<T, S>(
    data: &JobSchema,
    job: &dyn Job<State = S>,
    last_executed: DateTime<Utc>,
    registry_meta: &JobRegistryMeta<S>,
    result: Result<T, RunJobError>,
) -> Result<T, RunJobError>
where
    S: Clone + Send + Sync + 'static,
{
    result
        .attach_printable_lazy(|| format!("job.id = {:?}", data.id))
        .attach_printable_lazy(|| format!("job.created_at = {:?}", data.created_at))
        .attach_printable_lazy(|| format!("job.data.type = {:?}", registry_meta.kind))
        .attach_printable_lazy(|| format!("job.deadline = {:?}", data.deadline))
        .attach_printable_lazy(|| format!("job.failed_attempts = {:?}", data.failed_attempts))
        .attach_printable_lazy(|| format!("job.last_retry = {:?}", data.last_retry))
        .attach_printable_lazy(|| format!("job.priority = {:?}", data.priority))
        .attach_printable_lazy(|| format!("job.timeout = {:?}", job.timeout()))
        .attach_printable_lazy(|| format!("job.data = {:?}", job))
        .attach_printable_lazy(|| format!("last executed: {:?}", last_executed.to_rfc3339()))
}

fn serialize_job<J, S>(job: &J) -> Result<JobRawData, SerializeJobError>
where
    J: Job<State = S> + Serialize,
    S: Clone + Send + Sync + 'static,
{
    let data = serde_json::to_value(job).change_context(SerializeJobError)?;
    Ok(JobRawData {
        kind: J::kind().to_string(),
        data,
    })
}

pub async fn run_job<S>(
    runner: &JobRunner<S>,
    job: &dyn Job<State = S>,
    registry_meta: &JobRegistryMeta<S>,
) -> Result<JobStatus, RunJobError>
where
    S: Clone + Send + Sync + 'static,
{
    let job_future = job.run(runner.0.state.clone()).boxed();
    let job_future = CatchUnwindJobFuture::new(job_future);

    let timeout = job
        .timeout()
        .to_std()
        .change_context(RunJobError)
        .attach_printable_lazy(|| format!("job {:?}'s timeout is invalid", registry_meta.kind))?;

    tokio::time::timeout(timeout, job_future)
        .await
        .change_context(RunJobError)
        .attach(JobTimedOut)?
}

pub async fn clear_all_queued_jobs<S>(runner: &JobRunner<S>) -> Result<u64, ClearAllJobsError>
where
    S: Clone + Send + Sync + 'static,
{
    // go with transaction mode, it will revert back progress if it fails
    let mut conn = runner
        .0
        .pool
        .begin()
        .await
        .change_context(ClearAllJobsError)
        .attach_printable("could not start database transaction")?;

    let deleted = JobSchema::delete_all(&mut conn)
        .await
        .change_context(ClearAllJobsError)
        .attach_printable("could not clear all jobs into the database")?;

    conn.commit()
        .await
        .change_context(ClearAllJobsError)
        .attach_printable("could not commit database transaction")?;

    Ok(deleted)
}

pub async fn insert_into_queue_db<J, S>(
    runner: &JobRunner<S>,
    job: &J,
    schedule: Option<Schedule>,
) -> Result<(), QueueJobError>
where
    J: Job<State = S> + Serialize,
    S: Clone + Send + Sync + 'static,
{
    let now = Utc::now();
    let raw_data = serialize_job(job).change_context(QueueJobError)?;
    let deadline = schedule
        .map(|v| v.timestamp(Some(now)))
        .or_else(|| J::schedule().upcoming(Some(now)));

    let Some(deadline) = deadline else {
        return Err(eden_utils::Error::context(
            eden_utils::ErrorCategory::Unknown,
            QueueJobError,
        ))
        .attach_printable(format!(
            "job {:?} unable to get job deadline (required from the database)",
            J::kind()
        ));
    };

    let form = InsertJobForm::builder()
        .data(raw_data)
        .deadline(deadline)
        .priority(J::priority())
        .build();

    let mut conn = runner
        .0
        .pool
        .acquire()
        .await
        .change_context(QueueJobError)
        .attach_printable("could not get database connection")?;

    JobSchema::insert(&mut conn, form)
        .await
        .change_context(QueueJobError)
        .attach_printable("could not insert job into the database")?;

    Ok(())
}
