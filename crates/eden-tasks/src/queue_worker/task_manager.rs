use chrono::{DateTime, TimeDelta, Utc};
use eden_tasks_schema::forms::UpdateTaskForm;
use eden_tasks_schema::types::{Task, TaskPriority, TaskRawData, TaskStatus, WorkerId};
use eden_utils::error::exts::{AnonymizedResultExt, ResultExt};
use eden_utils::error::tags::Suggestion;
use eden_utils::Result;
use pin_project_lite::pin_project;
use std::future::Future;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::futures::Notified;
use tokio::sync::{Notify, Semaphore, SemaphorePermit};
use tokio::task::JoinHandle;
use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};
use tokio_util::task::task_tracker::TaskTrackerWaitFuture;
use tokio_util::task::TaskTracker;
use tracing::{debug, info, trace, warn, Instrument, Span};

use crate::error::PerformTaskError;
use crate::registry::{RecurringTask, RegistryItem};
use crate::{Scheduled, TaskRunContext};

use super::QueueWorker;

#[derive(Clone)]
pub struct QueueWorkerTaskManager(Arc<QueueWorkerTaskManagerInner>);

impl QueueWorkerTaskManager {
    #[must_use]
    pub fn new(concurrency: usize, id: WorkerId) -> Self {
        Self(Arc::new(QueueWorkerTaskManagerInner {
            aborted: CancellationToken::new(),
            close_token: CancellationToken::new(),
            changed_tasks_notify: Arc::new(Notify::new()),
            futures: TaskTracker::new(),
            id,
            pending_tasks: Arc::new(AtomicUsize::new(0)),
            running_tasks: Arc::new(AtomicUsize::new(0)),
            semaphore: Semaphore::new(concurrency),
        }))
    }
}

pub struct QueueWorkerTaskManagerInner {
    aborted: CancellationToken,
    close_token: CancellationToken,
    changed_tasks_notify: Arc<Notify>,
    futures: TaskTracker,
    id: WorkerId,
    pending_tasks: Arc<AtomicUsize>,
    running_tasks: Arc<AtomicUsize>,
    semaphore: Semaphore,
}

#[allow(unused)]
#[must_use = "It must be declared with a variable to remain its permit until it is dropped"]
pub struct WorkerPermitTaskGuard<'a> {
    notify: Arc<Notify>,
    pending_tasks: Arc<AtomicUsize>,
    permit: Option<SemaphorePermit<'a>>,
    running_tasks: Arc<AtomicUsize>,
}

pub enum PendingTask {
    Recurring {
        deadline: DateTime<Utc>,
        task: Arc<RecurringTask>,
    },
    Queued(Task),
}

impl QueueWorkerTaskManager {
    pub fn abort(&self) {
        self.aborted.cancel();
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.close_token.is_cancelled()
    }

    pub fn close(&self) {
        self.close_token.cancel();
        self.futures.close();
    }

    pub fn closed(&self) -> WaitForCancellationFuture<'_> {
        self.close_token.cancelled()
    }

    #[must_use]
    pub fn pending_tasks(&self) -> usize {
        self.pending_tasks.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn running_tasks(&self) -> usize {
        self.running_tasks.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn futures_len(&self) -> usize {
        self.futures.len()
    }

    #[must_use]
    pub fn wait_for_all_futures(&self) -> TaskTrackerWaitFuture<'_> {
        self.futures.wait()
    }

    pub fn running_tasks_changed(&self) -> RunningTasksChanged<'_> {
        RunningTasksChanged {
            fut: self.changed_tasks_notify.notified(),
            running_tasks: self.running_tasks.clone(),
        }
    }
}

impl QueueWorkerTaskManager {
    #[allow(clippy::expect_used)]
    pub fn handle_pending_task<S>(
        &self,
        now: DateTime<Utc>,
        worker: QueueWorker<S>,
        task: PendingTask,
    ) -> JoinHandle<()>
    where
        S: Clone + Send + Sync + 'static,
    {
        let manager = self.clone();

        let ctx = task.run_context(manager.id, now);
        let span = tracing::info_span!(
            "perform_task",
            task.id = %ctx.id,
            task.kind = ?task.kind(),
            task.created_at = %ctx.created_at,
            task.attempts = %ctx.attempts,
            task.data = tracing::field::Empty,
            task.deadline = %ctx.deadline,
            task.is_recurring = %task.is_recurring(),
            task.is_retrying = %ctx.is_retrying,
            task.rust_type = tracing::field::Empty,
        );

        self.futures.spawn(
            async move {
                let Some(_permit) = manager.permit_task().await else {
                    warn!("aborted awaiting task {:?} ({})", ctx.id, task.kind());
                    return;
                };
                let _guard = task.as_recurring_task().map(|v| v.running_guard());

                let (action, boxed_task) = manager.perform_task(&worker, &task, &ctx).await;
                let boxed_task = boxed_task.expect("unexpected boxed_task to be None");

                let is_completed = matches!(action, PerformTaskAction::Completed);
                let result = task
                    .handle_task_action(&ctx, boxed_task, &worker, action)
                    .await;

                if let Err(error) = result {
                    warn!(%error, "task {:?} failed to perform post-task action", ctx.id);
                    return;
                }

                // Unblock if it is periodic task, if nothing goes wrong
                let option = worker.0.registry.get_recurring_task(task.kind()).await;
                if let Some(task) = option
                    && task.is_blocked().await
                    && is_completed
                {
                    info!(
                        "unblocked recurring task {:?} ({}). allowing task to run periodically",
                        task.kind, task.rust_name
                    );
                    task.set_blocked(false).await;
                }
            }
            .instrument(span),
        )
    }

    async fn perform_task<S>(
        &self,
        worker: &QueueWorker<S>,
        task: &PendingTask,
        ctx: &TaskRunContext,
    ) -> (
        PerformTaskAction,
        Option<Box<dyn crate::Task<State = S> + 'static>>,
    )
    where
        S: Clone + Send + Sync + 'static,
    {
        let Some(item) = worker.0.registry.find_item(task.kind()) else {
            warn!("cannot find registry metadata for task {:?}", task.kind());
            return (PerformTaskAction::Delete, None);
        };
        let span = Span::current();
        span.record("task.rust_type", tracing::field::display(item.rust_name));

        let task = match task.try_deserialize_task(&item) {
            Ok(n) => n,
            Err(error) => {
                warn!(
                    error = %error.anonymize(),
                    "could not deserialize task for {:?} ({})",
                    task.kind(),
                    item.rust_name
                );
                return (PerformTaskAction::Delete, None);
            }
        };
        span.record("task.data", tracing::field::debug(&task));

        let result = worker.perform_task(&*task, ctx, &item).await;
        let action = match result {
            Ok(action) => action,
            Err(error) => {
                let error = error.anonymize();
                warn!(%error, "failed to perform task {:?}", item.kind);

                let action = error.get_attached_any().next().cloned();
                action.unwrap_or(PerformTaskAction::RetryOnError)
            }
        };

        (action, Some(task))
    }

    async fn permit_task(&self) -> Option<WorkerPermitTaskGuard<'_>> {
        trace!(
            "available semaphore permits = {}",
            self.semaphore.available_permits()
        );
        if self.semaphore.available_permits() == 0 {
            debug!("worker semaphore ran out of permits, waiting for task(s) to finish");
        }

        self.pending_tasks.fetch_add(1, Ordering::Relaxed);
        tokio::select! {
            result = self.semaphore.acquire() => {
                let permit = match result {
                    Ok(permit) => Some(permit),
                    Err(error) => {
                        warn!(%error, "failed to acquire permit from worker semaphore");
                        None
                    }
                };

                self.running_tasks.fetch_add(1, Ordering::Relaxed);
                self.changed_tasks_notify.notify_waiters();

                Some(WorkerPermitTaskGuard {
                    notify: self.changed_tasks_notify.clone(),
                    permit,
                    pending_tasks: self.pending_tasks.clone(),
                    running_tasks: self.running_tasks.clone(),
                })
            }
            // Do not continue if worker is shutting down
            _ = self.aborted.cancelled() => {
                self.pending_tasks.fetch_sub(1, Ordering::Relaxed);
                None
            }
        }
    }
}

impl Deref for QueueWorkerTaskManager {
    type Target = QueueWorkerTaskManagerInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Drop for WorkerPermitTaskGuard<'a> {
    fn drop(&mut self) {
        self.pending_tasks.fetch_sub(1, Ordering::Relaxed);
        self.running_tasks.fetch_sub(1, Ordering::Relaxed);
        self.notify.notify_waiters();
    }
}

pin_project! {
    #[must_use]
    pub struct RunningTasksChanged<'a> {
        #[pin]
        fut: Notified<'a>,
        running_tasks: Arc<AtomicUsize>,
    }
}

impl<'a> Future for RunningTasksChanged<'a> {
    type Output = usize;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        use std::task::Poll;

        let this = self.project();
        match this.fut.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(..) => Poll::Ready(this.running_tasks.load(Ordering::Relaxed)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerformTaskAction {
    Completed,
    Delete,
    RetryIn(TimeDelta),
    RetryOnError,
    RetryOnTimedOut,
}

impl PendingTask {
    fn as_recurring_task(&self) -> Option<&RecurringTask> {
        match self {
            Self::Recurring { task, .. } => Some(&task),
            _ => None,
        }
    }

    fn try_deserialize_task<S>(
        &self,
        registry_item: &RegistryItem<S>,
    ) -> Result<Box<dyn crate::Task<State = S>>, PerformTaskError>
    where
        S: Clone + Send + Sync + 'static,
    {
        let mut is_artifical = false;

        let deserializer = &*registry_item.deserializer;
        let raw_data = match self {
            Self::Queued(task) => task.data.clone(),
            Self::Recurring { task, .. } => {
                // generate fake info because why not
                is_artifical = true;
                TaskRawData {
                    kind: task.kind.into(),
                    inner: serde_json::Value::Null,
                }
            }
        };

        let mut result = deserializer(raw_data.inner)
            .map_err(|e| eden_utils::Error::any(eden_utils::ErrorCategory::Unknown, e))
            .change_context(PerformTaskError)
            .attach_printable_lazy(|| {
                format!("could not deserialize task {}", registry_item.rust_name)
            });

        if is_artifical {
            result = result.attach_lazy(|| {
                Suggestion::owned(format_args!(
                    "Make sure {} ({}) does not need any required data or it is a unit struct!",
                    registry_item.rust_name, registry_item.kind
                ))
            });
        }

        result
    }

    async fn handle_task_action<S>(
        &self,
        context: &TaskRunContext,
        task: Box<dyn crate::Task<State = S> + 'static>,
        worker: &QueueWorker<S>,
        result: PerformTaskAction,
    ) -> Result<()>
    where
        S: Clone + Send + Sync + 'static,
    {
        use PerformTaskAction::*;

        let mut conn = worker.db_connection().await?;
        let is_recurring = self.is_recurring();

        let (retry_in, attempts) = match self {
            Self::Recurring { task: info, .. } => match result {
                Completed => {
                    debug!("completed task {:?}", info.kind);

                    let now = Utc::now();
                    info.set_running(false);
                    info.update_deadline(now).await;
                    return Ok(());
                }
                Delete => return Ok(()),
                RetryIn(duration) => (duration, 0),
                RetryOnError | RetryOnTimedOut => (task.backoff(0), 0),
            },
            Self::Queued(info) => match result {
                Completed => {
                    debug!("completed task {:?}", info.data.kind);

                    let form = UpdateTaskForm::builder()
                        .status(Some(TaskStatus::Success))
                        .build();

                    return Task::update(&mut conn, context.id, form)
                        .await
                        .map(|_| ())
                        .anonymize_error();
                }
                Delete => {
                    debug!("deleted task for {:?}", info.data.kind);
                    return Task::delete(&mut conn, context.id)
                        .await
                        .map(|_| ())
                        .anonymize_error();
                }
                RetryIn(duration) => (
                    duration,
                    u16::try_from(context.attempts).unwrap_or(u16::MAX - 1),
                ),
                RetryOnError | RetryOnTimedOut => {
                    let attempts = u16::try_from(context.attempts).unwrap_or(u16::MAX - 1);
                    (task.backoff(attempts + 1), attempts)
                }
            },
        };

        if attempts + 1 > worker.0.max_attempts {
            warn!(
                attempts = %attempts,
                threshold = %worker.0.max_attempts,
                "task {:?} ran too many attempts; failing task...",
                self.kind(),
            );

            if !is_recurring {
                return Task::fail(&mut conn, context.id)
                    .await
                    .map(|_| ())
                    .anonymize_error();
            }
        }

        debug!("retrying task {:?} for {retry_in}", self.kind());

        let now = Utc::now();
        if !is_recurring {
            let scheduled = Scheduled::In(retry_in);
            return worker
                .requeue(context.id, Some(now), scheduled, attempts)
                .await
                .attach_printable_lazy(|| format!("could not requeue task for {}", context.id))
                .anonymize_error();
        }

        // Worker::queue will block the recurring task automatically
        let queue_result = worker.queue(
            Some(context.id),
            TaskRawData {
                kind: self.kind().into(),
                inner: serde_json::Value::Null,
            },
            Scheduled::In(retry_in),
            Some(now),
            1,
        );

        if let Err(error) = queue_result.await {
            tracing::warn!(
                error = %error.anonymize(),
                "could not queue recurring task {:?} ({})",
                context.id,
                self.kind()
            );
        } else {
            tracing::debug!(
                "queued recurring task {:?} ({}) for {retry_in}",
                context.id,
                self.kind()
            );
        }

        Ok(())
    }

    fn run_context(&self, worker_id: WorkerId, now: DateTime<Utc>) -> TaskRunContext {
        match self {
            Self::Queued(data) => TaskRunContext::from_task_schema(worker_id, data),
            Self::Recurring { deadline, .. } => {
                TaskRunContext::from_recurring(worker_id, *deadline, now)
            }
        }
    }

    #[must_use]
    pub fn kind(&self) -> &str {
        match self {
            Self::Recurring { task, .. } => task.kind,
            Self::Queued(task) => &task.data.kind,
        }
    }

    #[must_use]
    pub fn deadline(&self) -> DateTime<Utc> {
        match self {
            Self::Recurring { deadline, .. } => *deadline,
            Self::Queued(task) => task.deadline,
        }
    }

    #[must_use]
    pub fn is_recurring(&self) -> bool {
        matches!(self, Self::Recurring { .. })
    }

    #[must_use]
    pub fn priority(&self) -> TaskPriority {
        match self {
            Self::Recurring { task, .. } => task.priority(),
            Self::Queued(task) => task.priority,
        }
    }
}
