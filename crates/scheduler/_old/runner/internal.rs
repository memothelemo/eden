use crate::runner::catch_unwind::CatchUnwindTaskFuture;
use crate::runner::TaskTimedOut;
use crate::{Task, TaskResult, TaskRunner, TaskSchedule, Schedule};

use chrono::{DateTime, Utc};
use eden_db::forms::InsertTaskForm;
use eden_db::schema::{Task as TaskSchema, TaskRawData};
use eden_utils::error::ResultExt;
use eden_utils::Result;
use futures::FutureExt;
use serde::Serialize;
use serde_json::Value as Json;

use super::{ClearAllTasksError, QueueTaskError, RunTaskError, SerializeTaskError};

pub struct BuilderState;

pub struct TaskRegistryMeta<S> {
    pub(crate) deserializer: DeserializerFn<S>,
    pub(crate) kind: &'static str,
    pub(crate) schedule: ScheduleFn,
}

pub type DeserializerFn<State> =
    Box<dyn Fn(Json) -> serde_json::Result<Box<dyn Task<State = State>>> + Send + Sync + 'static>;

pub type ScheduleFn = Box<dyn Fn() -> TaskSchedule + Send + Sync + 'static>;

pub struct ProvidedTaskData;

pub fn provide_task_data_if_error<T, E, S>(
    data: &TaskSchema,
    task: Option<&dyn Task<State = S>>,
    last_executed: DateTime<Utc>,
    registry_meta: Option<&TaskRegistryMeta<S>>,
    result: Result<T, E>,
) -> Result<T, E>
where
    E: eden_utils::error::Context,
    S: Clone + Send + Sync + 'static,
{
    let mut result = match result {
        Ok(n) => return Ok(n),
        Err(error) if error.contains::<ProvidedTaskData>() => return Err(error),
        res => res,
    };

    result = result
        .attach_printable(format!("task.id = {:?}", data.id))
        .attach_printable(format!("task.created_at = {:?}", data.created_at))
        .attach_printable(format!("task.deadline = {:?}", data.deadline))
        .attach_printable(format!("task.failed_attempts = {:?}", data.failed_attempts))
        .attach_printable(format!("task.last_retry = {:?}", data.last_retry))
        .attach_printable(format!("task.priority = {:?}", data.priority))
        .attach_printable(format!("task.data = {:?}", task));

    if let Some(registry_meta) = registry_meta {
        result = result.attach_printable(format!("task.data.type = {:?}", registry_meta.kind))
    }

    if let Some(task) = task {
        result = result.attach_printable(format!("task.timeout = {:?}", task.timeout()))
    }

    result
        .attach_printable(format!("last executed: {:?}", last_executed.to_rfc3339()))
        .attach(ProvidedTaskData)
}

fn serialize_task<J, S>(task: &J) -> Result<TaskRawData, SerializeTaskError>
where
    J: Task<State = S> + Serialize,
    S: Clone + Send + Sync + 'static,
{
    let data = serde_json::to_value(task).change_context(SerializeTaskError)?;
    Ok(TaskRawData {
        kind: J::kind().to_string(),
        data,
    })
}

pub async fn run_task<S>(
    runner: &TaskRunner<S>,
    task: &dyn Task<State = S>,
    registry_meta: &TaskRegistryMeta<S>,
) -> Result<TaskResult, RunTaskError>
where
    S: Clone + Send + Sync + 'static,
{
    let task_future = task.run(runner.0.state.clone()).boxed();
    let task_future = CatchUnwindTaskFuture::new(task_future);

    let timeout = task
        .timeout()
        .to_std()
        .change_context(RunTaskError)
        .attach_printable_lazy(|| format!("task {:?}'s timeout is invalid", registry_meta.kind))?;

    tokio::time::timeout(timeout, task_future)
        .await
        .change_context(RunTaskError)
        .attach(TaskTimedOut)?
}

pub async fn clear_all_queued_tasks<S>(runner: &TaskRunner<S>) -> Result<u64, ClearAllTasksError>
where
    S: Clone + Send + Sync + 'static,
{
    // go with transaction mode, it will revert back progress if it fails
    let mut conn = runner
        .0
        .pool
        .begin()
        .await
        .change_context(ClearAllTasksError)
        .attach_printable("could not start database transaction")?;

    let deleted = TaskSchema::delete_all(&mut conn)
        .await
        .change_context(ClearAllTasksError)
        .attach_printable("could not clear all tasks into the database")?;

    conn.commit()
        .await
        .change_context(ClearAllTasksError)
        .attach_printable("could not commit database transaction")?;

    Ok(deleted)
}

pub async fn insert_into_queue_db<J, S>(
    runner: &TaskRunner<S>,
    task: &J,
    schedule: Option<Schedule>,
) -> Result<(), QueueTaskError>
where
    J: Task<State = S> + Serialize,
    S: Clone + Send + Sync + 'static,
{
    let now = Utc::now();
    let raw_data = serialize_task(task).change_context(QueueTaskError)?;
    let deadline = schedule
        .map(|v| v.timestamp(Some(now)))
        .or_else(|| J::schedule().upcoming(Some(now)));

    let Some(deadline) = deadline else {
        return Err(eden_utils::Error::context(
            eden_utils::ErrorCategory::Unknown,
            QueueTaskError,
        ))
        .attach_printable(format!(
            "task {:?} unable to get task deadline (required from the database)",
            J::kind()
        ));
    };

    let form = InsertTaskForm::builder()
        .data(raw_data)
        .deadline(deadline)
        .priority(J::priority())
        .build();

    let mut conn = runner
        .0
        .pool
        .acquire()
        .await
        .change_context(QueueTaskError)
        .attach_printable("could not get database connection")?;

    TaskSchema::insert(&mut conn, form)
        .await
        .change_context(QueueTaskError)
        .attach_printable("could not insert task into the database")?;

    Ok(())
}
