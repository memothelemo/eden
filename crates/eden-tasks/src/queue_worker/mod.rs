use eden_tasks_schema::types::TaskRawData;
use eden_utils::error::exts::{IntoTypedError, ResultExt};
use eden_utils::error::tags::Suggestion;
use eden_utils::sql::SqlErrorExt;
use eden_utils::time::IntoStdDuration;
use eden_utils::{Error, ErrorCategory, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use self::catch_unwind::CatchUnwindTaskFuture;
use self::inner::QueueWorkerInner;
use self::task_manager::{PerformTaskAction, QueueWorkerTaskManager};
use crate::error::tags::ScheduleTaskTag;
use crate::error::{ScheduleTaskError, TaskError, WorkerStartError};
use crate::registry::{RegistryItem, TaskRegistry};
use crate::settings::Settings;
use crate::{Scheduled, Task, TaskResult, TaskRunContext};

mod builder;
mod catch_unwind;
mod database;
mod inner;
mod runner;
mod task_manager;

pub use eden_tasks_schema::types::WorkerId;

/// In Eden task queue architecture, there will be assigned workers
/// to perform a task that is required. The queue system will equally
/// distribute to all workers.
///
/// A task can be assigned to a worker by diving the task ID by total
/// amount of workers set by the configuration and its remainder value
/// will be the worker ID that will run the task.
#[derive(Clone)]
pub struct QueueWorker<S>(Arc<QueueWorkerInner<S>>);

impl<S: Clone + Send + Sync + 'static> QueueWorker<S> {
    #[must_use]
    pub fn new(id: WorkerId, pool: sqlx::PgPool, settings: &Settings, state: S) -> Self {
        Self(Arc::new(QueueWorkerInner {
            id,
            registry: Arc::new(TaskRegistry::new()),

            pool,
            runner_handle: Mutex::new(None),
            state,
            task_manager: QueueWorkerTaskManager::new(settings.max_running_tasks.get(), id),

            max_attempts: settings.max_task_retries,
            max_running_tasks: settings.max_running_tasks.get(),
            queued_tasks_per_batch: settings.queued_tasks_per_batch.get(),
            stalled_tasks_threshold: settings.stalled_tasks_threshold,
        }))
    }

    #[must_use]
    pub fn id(&self) -> WorkerId {
        self.0.id
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        // Assume it is running if it is already locked
        self.0
            .runner_handle
            .try_lock()
            .map(|v| v.is_some())
            .unwrap_or(true)
    }

    #[must_use]
    pub fn running_tasks(&self) -> usize {
        self.0.task_manager.running_tasks()
    }

    // strictly for testing only!
    #[doc(hidden)]
    #[must_use]
    pub fn get_state(&self) -> &S {
        &self.0.state
    }
}

impl<S: Clone + Send + Sync + 'static> QueueWorker<S> {
    #[allow(clippy::unwrap_used)]
    #[must_use]
    pub fn register_task<T>(self) -> Self
    where
        T: Task<State = S> + DeserializeOwned,
    {
        assert!(
            !self.is_running(),
            "registering task while the worker is running is not allowed!"
        );
        self.0.registry.register_task::<T>();
        self
    }

    /// Attempts to schedule a custom task into the queue
    /// to be ran in a later time.
    ///
    /// Recurring tasks ([`Task::trigger`](crate::Task::trigger) being returned as
    /// [`TaskTrigger::None`](crate::task::TaskTrigger::None)) are not allowed to be
    /// scheduled. It can be scheduled if it fails to do in time or encountered an
    /// error during the operation.
    ///
    /// It returns the queued job's id as [UUID](Uuid) to be referenced if needed.
    pub async fn schedule<T>(
        &self,
        task: T,
        scheduled: Scheduled,
    ) -> Result<Uuid, ScheduleTaskError>
    where
        T: crate::Task<State = S> + Serialize,
    {
        // Recurring tasks are not allowed to schedule unless
        // internally called from a secret function.
        if T::trigger().is_recurring() {
            return Err(Error::context(ErrorCategory::Unknown, ScheduleTaskError))
                .attach_printable("recurring tasks are not allowed to be scheduled")
                .attach_lazy(|| ScheduleTaskTag::new(&task));
        }

        let raw_data = TaskRawData {
            kind: T::kind().into(),
            inner: serde_json::to_value(&task)
                .into_typed_error()
                .change_context(ScheduleTaskError)
                .attach_printable("could not serialize task data")
                .attach_lazy(|| ScheduleTaskTag::new(&task))?,
        };

        self.queue(None, raw_data, scheduled, None, 0)
            .await
            .attach_lazy(|| ScheduleTaskTag::new(&task))
    }

    /// It starts processing incoming queued tasks indefinitely in a
    /// different thread until a shutdown signal is triggered.
    #[tracing::instrument(skip_all, level = "debug", fields(worker.id = %self.0.id))]
    pub async fn start(&self) -> Result<(), WorkerStartError> {
        let mut handle = self.0.runner_handle.lock().await;
        if handle.is_some() {
            return Err(Error::context(ErrorCategory::Unknown, WorkerStartError))
                .attach_printable("already started worker process");
        }

        // The database maybe offline for a while so we need to setup later on. :)
        let result = self.setup().await;

        let mut setup_later = result.is_err() && result.is_pool_error();
        if setup_later {
            warn!(
                "starting queue worker {} with an unhealthy database",
                self.0.id
            );
            setup_later = true;
        } else if result.is_err() {
            result?;
            debug!("starting queue worker {}", self.0.id);
        }

        let registry = &self.0.registry;
        registry.update_recurring_tasks_deadline(None).await;

        let worker_tx = self.clone();
        *handle = Some(eden_utils::tokio::spawn(
            "eden_tasks::worker::runner::run",
            self::runner::QueueWorkerRunner::new(worker_tx, setup_later).run(),
        ));

        Ok(())
    }

    #[allow(clippy::let_underscore_must_use)]
    #[tracing::instrument(skip_all, fields(worker.id = %self.id()))]
    pub async fn shutdown(&self) {
        // Couple of checks so we don't need to request for shutdown many times
        if !self.is_running() {
            return;
        }

        let task_manager = &self.0.task_manager;
        if task_manager.is_closed() {
            return;
        }

        // Real shutdown happens
        info!("shutting down queue worker {}", self.id());
        task_manager.close();

        // We want to wait for tasks to be finished
        let mut abort = false;
        loop {
            if abort {
                break;
            }

            let running_tasks = task_manager.running_tasks();
            let pending_tasks = task_manager.pending_tasks();
            if running_tasks == 0 && pending_tasks == 0 {
                info!("all task(s) are finished");
                break;
            }

            info!("waiting for {pending_tasks} task(s) to finish...");
            tokio::select! {
                _ = task_manager.running_tasks_changed() => {}
                _ = eden_utils::shutdown::aborted() => {
                    debug!("aborting all pending and running task(s)");
                    task_manager.abort();
                    abort = true;
                }
            }
        }

        // Waiting for the runner thread and other futures to finish
        let mut handle = self.0.runner_handle.lock().await;
        if !abort && let Some(handle) = handle.take() {
            task_manager.wait_for_all_futures().await;
            let _ = handle.await;
        }
    }
}

impl<S: Clone + Send + Sync + 'static> QueueWorker<S> {
    async fn perform_task(
        &self,
        task: &(dyn Task<State = S> + 'static),
        ctx: &TaskRunContext,
        registry_item: &RegistryItem<S>,
    ) -> Result<PerformTaskAction, TaskError> {
        let future = task.perform(ctx, self.0.state.clone());
        let future = CatchUnwindTaskFuture::new(future);

        let timeout = task
            .timeout()
            .into_std_duration()
            .ok_or_else(|| Error::context(ErrorCategory::Unknown, TaskError))
            .attach_printable_lazy(|| {
                format!(
                    "could not get task timeout for {:?} ({})",
                    registry_item.kind, registry_item.rust_name
                )
            })
            .attach(Suggestion::new(
                "Try to make the return value of Task::timeout not a negative duration",
            ))
            .attach(PerformTaskAction::Delete)?;

        debug!("performing task {:?}", registry_item.kind);

        let result = tokio::time::timeout(timeout, future)
            .await
            .into_typed_error()
            .change_context(TaskError)
            .attach(PerformTaskAction::RetryOnTimedOut)
            .flatten();

        let action = match result {
            Ok(TaskResult::Completed) => PerformTaskAction::Completed,
            Ok(TaskResult::RetryIn(n)) => PerformTaskAction::RetryIn(n),
            Ok(TaskResult::Reject(error)) => {
                warn!(
                    error = %error,
                    "task {:?} got a rejection error",
                    registry_item.kind
                );
                PerformTaskAction::Delete
            }
            Err(error) => {
                let error = error.anonymize();
                tracing::error!(
                    error = %error,
                    "task {:?} got an error",
                    registry_item.kind,
                );

                let action = error
                    .get_attached_any::<PerformTaskAction>()
                    .next()
                    .cloned();

                action.unwrap_or(PerformTaskAction::RetryOnError)
            }
        };

        trace!("done performing task {:?}", registry_item.kind);
        Ok(action)
    }
}
