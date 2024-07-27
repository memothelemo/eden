use chrono::{DateTime, Utc};
use dashmap::DashMap;
use eden_db::forms::InsertTaskForm;
use eden_db::schema::{Task, TaskRawData, TaskStatus};
use eden_utils::error::{AnyResultExt, ErrorExt, ResultExt};
use eden_utils::{Error, ErrorCategory, Result};
use futures::FutureExt;
use serde::Serialize;
use sqlx::{pool::PoolConnection, Transaction};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use uuid::Uuid;

mod builder;
mod catch_unwind;
mod config;
mod periodic;
mod registry;
mod runner;

use crate::{error::*, Task as AnyTask, TaskResult};
use crate::{Scheduled, TaskPerformInfo};

use self::builder::BuilderState;
use self::catch_unwind::CatchUnwindTaskFuture;
use self::periodic::PeriodicTask;
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
        for periodic_task in self.0.periodic_tasks.read().await.iter() {
            periodic_task.set_blocked(false).await;
        }

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

        let task = Task::delete(&mut conn, id)
            .await
            .change_context(DeleteTaskError)
            .attach_printable_lazy(|| format!("task.id = {id:?}"))?;

        // unblock if it is a periodic task
        if let Some(task) = task.as_ref() {
            if let Some(periodic_task) = self.get_periodic_task(&task.data.kind).await {
                periodic_task.set_blocked(false).await;
            }
        }

        Ok(task.is_some())
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

        self.queue(None, false, task_data, scheduled, None)
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
            // TODO: log errors from handle
            let _ = handle.await;
        }
    }

    /// Processes incoming tasks indefinitely.
    pub async fn start(&self) -> Result<(), StartQueueError> {
        let mut handle = self.0.runner_handle.lock().await;
        if handle.is_some() {
            return Err(Error::context(ErrorCategory::Unknown, StartQueueError))
                .attach_printable("already started processing incoming tasks");
        }

        self.update_periodic_tasks_blacklist()
            .await
            .transform_context(StartQueueError)
            .attach_printable("could not update periodic tasks blacklist")?;

        // Initialize all periodic tasks to have their own deadlines
        let now = Utc::now();
        let periodic_tasks = self.0.periodic_tasks.read().await;
        for task in periodic_tasks.iter() {
            task.adjust_deadline(now).await;
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

    async fn update_periodic_tasks_blacklist(&self) -> Result<()> {
        let mut conn = self.db_connection().await?;

        let mut stream = Task::get_all().periodic(true).build().size(50);
        while let Some(tasks) = stream.next(&mut conn).await.anonymize_error()? {
            for task in tasks {
                if let Some(task) = self.get_periodic_task(&task.data.kind).await {
                    eprintln!("{:?} is blocked", task.task_type);
                    task.set_blocked(true).await;
                } else {
                    eprintln!("unknown periodic task: {:?}", task.data.kind);
                }
            }
        }

        Ok(())
    }

    async fn get_periodic_task(&self, task_type: &str) -> Option<Arc<PeriodicTask>> {
        let tasks = self.0.periodic_tasks.read().await;
        tasks.iter().find(|v| v.task_type == task_type).cloned()
    }

    /// Unsafe version of [`Queue::schedule`] but any registered task
    /// (periodic or persistent) can be scheduled.
    async fn queue(
        &self,
        id: Option<Uuid>,
        is_periodic: bool,
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
            if let Some(task) = self.get_periodic_task(&registry_meta.kind).await {
                task.set_blocked(true).await;
            }
        }

        let form = InsertTaskForm::builder()
            .id(id)
            .data(raw_data)
            .deadline(deadline)
            .periodic(is_periodic)
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

    /// Performs any task, regardless queued or periodic
    async fn perform_task(
        &self,
        task: &(dyn AnyTask<State = S> + 'static),
        perform_info: &TaskPerformInfo,
        registry_meta: &TaskRegistryMeta<S>,
    ) -> Result<PerformTaskAction, PerformTaskError> {
        println!(
            "running task {} ({}); data = {task:?}",
            perform_info.id, registry_meta.kind
        );

        let task_future = task.perform(perform_info, self.0.state.clone()).boxed();
        let task_future = CatchUnwindTaskFuture::new(task_future);

        let timeout = runner::resolve_time_delta(task.timeout())
            .ok_or_else(|| Error::context(ErrorCategory::Unknown, PerformTaskError))
            .attach_printable_lazy(|| {
                format!("could not get task timeout for {:?}", registry_meta.kind)
            })
            .attach(PerformTaskAction::Delete)?;

        let result = tokio::time::timeout(timeout, task_future)
            .await
            .change_context(PerformTaskError)
            .map_err(|e| e.attach(PerformTaskAction::RetryOnError))
            .flatten();

        let action = match result {
            Ok(TaskResult::Completed) => PerformTaskAction::Completed,
            Ok(TaskResult::RetryIn(n)) => PerformTaskAction::RetryIn(n),
            Ok(TaskResult::Fail(n)) => {
                eprintln!(
                    "task {} with type {:?}; task got a fatal error: {n}",
                    perform_info.id, registry_meta.kind,
                );
                PerformTaskAction::Delete
            }
            Err(error) => {
                let error = error.anonymize();
                eprintln!(
                    "task {} with type {:?}; task got an error: {error}",
                    perform_info.id, registry_meta.kind,
                );
                PerformTaskAction::RetryOnError
            }
        };

        Ok(action)
    }

    #[allow(unused)]
    fn spawn_fut<F>(&self, task: F)
    where
        F: futures::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.0.running_tasks.spawn(task);
    }

    #[allow(clippy::unused_self)]
    fn deserialize_task(
        &self,
        raw_data: &TaskRawData,
        registry_meta: &TaskRegistryMeta<S>,
    ) -> Result<Box<dyn AnyTask<State = S>>, PerformTaskError> {
        let deserializer = &*registry_meta.deserializer;
        let task = deserializer(raw_data.inner.clone())
            .map_err(|e| Error::any(ErrorCategory::Unknown, e))
            .transform_context(PerformTaskError)
            .attach_printable_lazy(|| {
                format!("could not deserialize task for {:?}", registry_meta.kind)
            })?;

        Ok(task)
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

/// What the scheduler should do after a task is performed
#[derive(Debug, Clone, Copy)]
enum PerformTaskAction {
    Delete,
    Completed,
    RetryIn(chrono::TimeDelta),
    RetryOnError,
    RetryOnTimedOut,
}

struct QueueInner<S> {
    // periodic tasks that are blocked from running (maybe it is already running
    // or being scheduled from the database because of an error)
    config: QueueConfig,
    periodic_tasks: Arc<RwLock<Vec<Arc<PeriodicTask>>>>,
    pool: sqlx::PgPool,
    registry: Arc<DashMap<&'static str, TaskRegistryMeta<S>>>,
    runner_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    running_tasks: TaskTracker,
    shutdown: CancellationToken,
    state: S,
}
