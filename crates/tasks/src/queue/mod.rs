use chrono::{DateTime, Utc};
use dashmap::DashMap;
use eden_db::forms::InsertTaskForm;
use eden_db::schema::{Task, TaskRawData, TaskStatus};
use eden_utils::error::{AnyResultExt, ResultExt};
use eden_utils::{Error, ErrorCategory, Result};
use serde::Serialize;
use sqlx::{pool::PoolConnection, Transaction};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use uuid::Uuid;

mod builder;
mod catch_unwind;
mod config;
mod error_tags;
mod registry;
mod runner;

use crate::Scheduled;
use crate::{error::*, Task as AnyTask};

use self::builder::BuilderState;
use self::registry::TaskRegistryMeta;

pub use self::config::*;

#[allow(private_interfaces)]
#[derive(Clone)]
pub struct Queue<S = BuilderState>(pub(crate) Arc<QueueInner<S>>);

impl<S> Queue<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Attempts to clear all queued tasks from the database.
    ///
    /// If it fails, this operation will revert back before the
    /// deletion of all available tasks.
    ///
    /// It returns the total amount of tasks deleted from
    /// the database.
    pub async fn clear_all(&self) -> Result<u64, ClearAllTasksError> {
        let mut conn = self
            .db_transaction()
            .await
            .transform_context(ClearAllTasksError)?;

        let deleted = Task::delete_all(&mut conn)
            .await
            .change_context(ClearAllTasksError)?;

        conn.commit()
            .await
            .change_context(ClearAllTasksError)
            .attach_printable("could not commit database transaction")?;

        // clear all blocked tasks if necessary and not running
        todo!("unblock all periodic tasks");

        Ok(deleted)
    }

    /// Attempts to clear all queued tasks from the database
    /// with given status only.
    ///
    /// If it fails, this operation will revert back before the
    /// deletion of all available tasks.
    ///
    /// It returns the total amount of tasks deleted from
    /// the database.
    pub async fn clear_all_with(&self, status: TaskStatus) -> Result<u64, ClearAllTasksError> {
        let mut conn = self
            .db_transaction()
            .await
            .transform_context(ClearAllTasksError)?;

        let deleted = Task::delete_all_with_status(&mut conn, status)
            .await
            .change_context(ClearAllTasksError)
            .attach_printable_lazy(|| format!("with status: {status:?}"))?;

        conn.commit()
            .await
            .change_context(ClearAllTasksError)
            .attach_printable("could not commit database transaction")?;

        Ok(deleted)
    }

    pub async fn delete_queued_task(&self, id: Uuid) -> Result<bool, DeleteTaskError> {
        let mut conn = self
            .db_connection()
            .await
            .transform_context(DeleteTaskError)?;

        Task::delete(&mut conn, id)
            .await
            .change_context(DeleteTaskError)
            .attach_printable_lazy(|| format!("task.id = {id:?}"))
            .map(|v| v.is_some())
    }

    pub async fn is_running(&self) -> bool {
        self.0.runner_handle.lock().await.is_some()
    }

    /// Attempts to schedule a custom task into the queue
    /// to be ran in a later time.
    ///
    /// Periodic tasks ([`Task::schedule`](AnyTask::schedule) being returned as [`TaskSchedule::Once`](crate::TaskSchedule::Once))
    /// are not allowed to be scheduled. It can be scheduled if it fails to do in time
    /// or encountered an error during the operation.
    ///
    /// It returns the queued job's id as [UUID](Uuid) to be referenced if needed.
    pub async fn schedule<T>(
        &self,
        task: T,
        scheduled: Scheduled,
    ) -> Result<Uuid, ScheduleTaskError>
    where
        T: AnyTask<State = S> + Serialize,
    {
        // Periodic tasks are not allowed to schedule unless
        // internally called from a secret function.
        //
        // I know these two lines are noisy but this is essential if
        // you're trying to investigate an error (it saves time).
        if T::schedule().is_periodic() {
            return Err(Error::context(ErrorCategory::Unknown, ScheduleTaskError))
                .attach_printable("periodic tasks are not allowed to be scheduled")
                .attach_printable_lazy(|| format!("task.type = {}", T::task_type()))
                .attach_printable_lazy(|| format!("task.data = {task:?}"));
        }

        let task_data = self
            .serialize_task(&task)
            .attach_printable_lazy(|| format!("task.type = {}", T::task_type()))
            .attach_printable_lazy(|| format!("task.data = {task:?}"))?;

        self.queue(task_data, scheduled, None)
            .await
            .attach_printable_lazy(|| format!("task.type = {}", T::task_type()))
            .attach_printable_lazy(|| format!("task.data = {task:?}"))
    }

    /// Attempts to shut down the queue runner and waits for all running tasks
    /// to be terminated regardless of their result.
    #[allow(clippy::let_underscore_must_use)]
    pub async fn shutdown(&self) {
        self.0.shutdown.cancel();
        self.0.running_tasks.close();
        self.0.running_tasks.wait().await;

        // wait for the runner handle to be terminated as well
        let mut handle = self.0.runner_handle.lock().await;
        if let Some(handle) = handle.take() {
            // TOOD: log errors from handle
            let _ = handle.await;
        }
    }

    /// Processes incoming tasks indefinitely.
    pub async fn start(&self) -> Result<(), AlreadyStartedError> {
        let mut handle = self.0.runner_handle.lock().await;
        if handle.is_some() {
            return Err(Error::context(ErrorCategory::Unknown, AlreadyStartedError));
        }
        *handle = Some(tokio::spawn(runner::runner(self.clone())));
        drop(handle);

        Ok(())
    }
}

impl<S> Queue<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Tries to establish database connection
    ///
    /// Refer to [sqlx's `PoolConnection` object](PoolConnection) for more documentation
    /// and how it should be used.
    pub async fn db_connection(&self) -> Result<PoolConnection<sqlx::Postgres>> {
        let pool = &self.0.pool;
        pool.acquire()
            .await
            .anonymize_error()
            .attach_printable("unable to establish connection to the database")
    }

    /// Tries to establish database transaction.
    ///
    /// Refer to [sqlx's Transaction object](Transaction) for more documentation
    /// and how it should be used.
    pub async fn db_transaction(&self) -> Result<Transaction<'_, sqlx::Postgres>> {
        let pool = &self.0.pool;
        pool.begin()
            .await
            .anonymize_error()
            .attach_printable("unable to start transaction from the database")
    }

    /// Unsafe version of [`Queue::schedule`] but any registered task
    /// (periodic or persistent) can be scheduled.
    async fn queue(
        &self,
        raw_data: TaskRawData,
        scheduled: Scheduled,
        now: Option<DateTime<Utc>>,
    ) -> Result<Uuid, ScheduleTaskError> {
        // Checking if this specified task is registered in the registry
        let Some(registry_meta) = self.0.registry.get(raw_data.kind.as_str()) else {
            return Err(Error::context(ErrorCategory::Unknown, ScheduleTaskError))
                .attach_printable(format!(
                    "task {:?} is not registered in the registry",
                    raw_data.kind
                ));
        };

        // Block this task from running it locally regardless if it reaches the deadline
        // (if it is a periodic task)
        let deadline = scheduled.timestamp(now);
        let priority = (*registry_meta.priority)();
        if registry_meta.is_periodic {
            todo!()
        }

        let form = InsertTaskForm::builder()
            .data(raw_data)
            .deadline(deadline)
            .priority(priority)
            .build();

        let mut conn = self
            .db_connection()
            .await
            .transform_context(ScheduleTaskError)?;

        let queued_task = Task::insert(&mut conn, form)
            .await
            .change_context(ScheduleTaskError)
            .attach_printable("could not insert task into the database")?;

        Ok(queued_task.id)
    }

    #[allow(clippy::unused_self)]
    fn serialize_task<T>(&self, task: &T) -> Result<TaskRawData, ScheduleTaskError>
    where
        T: AnyTask<State = S> + Serialize,
    {
        let data = serde_json::to_value(task)
            .change_context(ScheduleTaskError)
            .attach_printable("could not serialize task data")?;

        Ok(TaskRawData {
            kind: T::task_type().into(),
            inner: data,
        })
    }
}

impl<S> std::fmt::Debug for Queue<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskRunner")
            .field("config", &self.0.config)
            .field("registered_tasks", &self.0.registry.len())
            .field("state", &std::any::type_name::<S>())
            .finish()
    }
}

struct QueueInner<S> {
    // periodic tasks that are blocked from running (maybe it is already running
    // or being scheduled from the database because of an error)
    config: QueueConfig,
    pool: sqlx::PgPool,
    registry: Arc<DashMap<&'static str, TaskRegistryMeta<S>>>,
    runner_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    running_tasks: TaskTracker,
    shutdown: CancellationToken,
    state: S,
}