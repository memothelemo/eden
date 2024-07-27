use chrono::{DateTime, TimeDelta, Utc};
use eden_db::forms::UpdateTaskForm;
use eden_db::schema::{Task, TaskRawData, TaskStatus};
use eden_utils::error::{AnyResultExt, ErrorExt, ResultExt};
use eden_utils::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use thiserror::Error;
use tokio::sync::SemaphorePermit;
use tracing::Instrument;
use uuid::Uuid;

use super::{periodic::PeriodicTask, PerformTaskAction, Queue};
use crate::{Scheduled, TaskPerformInfo};

#[derive(Debug, Error)]
#[error("could not successfully run all pending queued task(s)")]
struct RunPendingQueuedTaskError;

impl<S> Queue<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Checks the semaphore status of the queueing system. If it reaches the
    /// maximum concurrent threads, pause for a bit then perform a task once
    /// the queue's semaphore's permits reach the maximum concurrent threads.
    async fn permit_process_task(&self) -> Option<SemaphorePermit<'_>> {
        if self.0.semaphore.available_permits() == 0 {
            tracing::debug!(
                "queue semaphore ran out of permits, waiting for task(s) to be finished"
            );
        }

        let result = self.0.semaphore.acquire().await.ok();
        tracing::debug!("queue semaphore permitted task to run");
        result
    }

    async fn perform_queued_task_inner(&self, task_info: &Task) -> PerformTaskAction {
        let Some(registry_meta) = self.0.registry.get(task_info.data.kind.as_str()) else {
            tracing::warn!(
                "cannot find registry metadata for task {:?}",
                task_info.data.kind
            );
            return PerformTaskAction::Delete;
        };

        let perform_info = TaskPerformInfo {
            id: task_info.id,
            created_at: task_info.created_at,
            deadline: task_info.deadline,
            attempts: task_info.attempts,
            last_retry: task_info.last_retry,
            is_retrying: task_info.attempts > 0,
        };

        let _permit = self.permit_process_task().await;
        self.0.running_tasks.fetch_add(1, Ordering::SeqCst);
        self.0.running_tasks_notify.notify_waiters();

        let task = match self.deserialize_task(&task_info.data, &registry_meta) {
            Ok(n) => n,
            Err(error) => {
                tracing::warn!(
                    error = %error.anonymize(),
                    "could not deserialize task for {:?}",
                    task_info.data.kind
                );
                return PerformTaskAction::Delete;
            }
        };

        let result = self
            .perform_task(&*task, &perform_info, &registry_meta)
            .await;

        let action = match result {
            Ok(action) => action,
            Err(error) => {
                let error = error.anonymize();
                tracing::warn!(
                    error = %error,
                    "failed to perform task for {:?}",
                    task_info.data.kind,
                );
                error
                    .get_attached_any()
                    .next()
                    .cloned()
                    .unwrap_or(PerformTaskAction::RetryOnError)
            }
        };

        self.0.running_tasks.fetch_sub(1, Ordering::SeqCst);
        self.0.running_tasks_notify.notify_waiters();

        // improvise actions with RetryOnError and RetryOnTimedOut into RetryIn since
        // we cannot access the deserialized task anymore
        match action {
            PerformTaskAction::RetryOnError | PerformTaskAction::RetryOnTimedOut => {
                let attempts = u16::try_from(task_info.attempts).unwrap_or_else(|_| 0);
                PerformTaskAction::RetryIn(task.backoff(attempts + 1))
            }
            _ => action,
        }
    }

    async fn perform_queued_task(&self, task: Task) {
        let max_attempts = self.0.config.max_attempts as i32;
        let action = self.perform_queued_task_inner(&task).await;

        let mut conn = match self.db_connection().await {
            Ok(conn) => conn,
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "could not establish database connection for task {:?}",
                    task.data.kind,
                );
                return;
            }
        };

        let now = Utc::now();
        let result = match action {
            PerformTaskAction::Delete => {
                tracing::debug!("deleted task for {:?}", task.data.kind);
                Task::delete(&mut conn, task.id)
                    .await
                    .map(|_| ())
                    .anonymize_error()
            }
            PerformTaskAction::Completed => {
                tracing::debug!("completed task for {:?}", task.data.kind);

                let form = UpdateTaskForm::builder()
                    .status(Some(TaskStatus::Success))
                    .build();

                Task::update(&mut conn, task.id, form)
                    .await
                    .map(|_| ())
                    .anonymize_error()
            }
            PerformTaskAction::RetryIn(delta) => {
                let total_attempts = task.attempts + 1;
                if total_attempts > max_attempts {
                    tracing::debug!(
                        "task {:?} received too many attempts; failing task",
                        task.data.kind,
                    );
                    Task::fail(&mut conn, task.id)
                        .await
                        .map(|_| ())
                        .anonymize_error()
                } else {
                    tracing::debug!("retrying task {:?} for {delta:?}", task.data.kind,);
                    self.requeue(task.id, Scheduled::In(delta), Some(now), task.attempts + 1)
                        .await
                        .anonymize_error()
                }
            }
            PerformTaskAction::RetryOnError | PerformTaskAction::RetryOnTimedOut => unreachable!(),
        };

        if let Err(error) = result {
            tracing::warn!(
                %error,
                "task {:?} failed to perform post-perform operation",
                task.data.kind
            );
            return;
        }

        // Unblock if it is a periodic task
        if let Some(periodic_task) = self.get_periodic_task(&task.data.kind).await {
            tracing::warn!(
                "task {:?} is a periodic task. allowing task to run periodically",
                task.data.kind
            );
            periodic_task.set_blocked(false).await;
        }
    }

    async fn run_pending_queued_tasks(
        &self,
        ticked_at: DateTime<Utc>,
    ) -> Result<(), RunPendingQueuedTaskError> {
        // First thing, requeue timed out jobs
        let mut conn = self
            .db_connection()
            .await
            .transform_context(RunPendingQueuedTaskError)?;

        // not much data is lost if converted from u16 to i32
        let max_attempts = self.0.config.max_attempts as i32;
        let stalled_tasks = Task::requeue_stalled(
            &mut conn,
            self.0.config.stalled_tasks_threshold,
            Some(ticked_at),
        )
        .await
        .change_context(RunPendingQueuedTaskError)?;

        if stalled_tasks > 0 {
            tracing::debug!("requeued {stalled_tasks} stalled tasks");
        } else {
            tracing::trace!("requeued {stalled_tasks} stalled tasks");
        }

        let mut stream = Task::pull_all_pending(max_attempts, Some(ticked_at)).size(50);
        while let Some(tasks) = stream
            .next(&mut conn)
            .await
            .change_context(RunPendingQueuedTaskError)?
        {
            for task in tasks {
                let queue_tx = self.clone();
                let span = tracing::info_span!(
                    "perform_queued_task",
                    task.id = %task.id,
                    task.created_at = %task.created_at,
                    task.deadline = %task.deadline,
                    task.periodic = %task.periodic,
                    task.attempts = %task.attempts,
                    "task.type" = %task.data.kind
                );
                let fut = async move { queue_tx.perform_queued_task(task).instrument(span).await };
                self.spawn_fut(fut);
            }
        }

        Ok(())
    }
}

impl<S> Queue<S>
where
    S: Clone + Send + Sync + 'static,
{
    #[tracing::instrument(skip_all)]
    async fn perform_periodic_task(&self, ticked_at: DateTime<Utc>, task: Arc<PeriodicTask>) {
        let Some(registry_meta) = self.0.registry.get(task.task_type) else {
            panic!(
                "Cannot find task registry metadata for periodic task {:?}",
                task.task_type
            );
        };

        // Generate a fake queued job data
        let fake_queued_task_id = Uuid::new_v4();
        let perform_info = TaskPerformInfo {
            id: fake_queued_task_id,
            created_at: ticked_at,
            deadline: task.deadline().await.unwrap_or(ticked_at),
            attempts: 0,
            last_retry: None,
            is_retrying: false,
        };

        let raw_data = TaskRawData {
            kind: task.task_type.into(),
            inner: serde_json::Value::Null,
        };

        // Don't forget to record this, these are already created
        let span = tracing::Span::current();
        if !span.is_disabled() {
            span.record("task.id", tracing::field::display(&perform_info.id));
            span.record(
                "task.created_at",
                tracing::field::display(&perform_info.created_at),
            );
            span.record(
                "task.attempts",
                tracing::field::display(&perform_info.attempts),
            );
            span.record(
                "task.deadline",
                tracing::field::display(&perform_info.deadline),
            );
            span.record("task.kind", tracing::field::display(registry_meta.kind));
            span.record(
                "task.is_retrying",
                tracing::field::display(perform_info.is_retrying),
            );
            span.record(
                "task.periodic",
                tracing::field::display(registry_meta.is_periodic),
            );
        }

        let _permit = self.permit_process_task().await;
        task.set_running(true);

        self.0.running_tasks.fetch_add(1, Ordering::SeqCst);
        self.0.running_tasks_notify.notify_waiters();

        let deserialized_task = match self.deserialize_task(&raw_data, &registry_meta) {
            Ok(n) => n,
            Err(error) => {
                let error = error
                    .attach_printable("suggestion: do not put any data needed for periodic tasks")
                    .anonymize();

                panic!(
                    "could not deserialize periodic task ({:?}) {error:#?}",
                    registry_meta.kind
                );
            }
        };

        let result = self
            .perform_task(&*deserialized_task, &perform_info, &registry_meta)
            .await;

        let action = match result {
            Ok(action) => action,
            Err(error) => {
                let error = error.anonymize();
                tracing::warn!(
                    error = %error,
                    "failed to perform periodic task for {:?}",
                    registry_meta.kind,
                );

                error
                    .get_attached_any()
                    .next()
                    .cloned()
                    .unwrap_or(PerformTaskAction::RetryOnError)
            }
        };

        let retry_in = match action {
            PerformTaskAction::Delete => return,
            PerformTaskAction::Completed => {
                let now = Utc::now();
                task.set_running(false);
                task.adjust_deadline(now).await;
                return;
            }
            PerformTaskAction::RetryIn(n) => n,
            PerformTaskAction::RetryOnError | PerformTaskAction::RetryOnTimedOut => {
                deserialized_task.backoff(0)
            }
        };

        let queue_result = self
            .queue(
                Some(perform_info.id),
                true,
                raw_data,
                Scheduled::In(retry_in),
                None,
                1,
            )
            .await;

        if let Err(error) = queue_result {
            tracing::warn!(
                error = %error.anonymize(),
                "could not queue periodic task for {:?}",
                registry_meta.kind,
            );
        } else {
            tracing::debug!(
                "queued periodic task {:?} for {retry_in}",
                registry_meta.kind,
            );
            task.set_blocked(true).await;
        }

        self.0.running_tasks.fetch_sub(1, Ordering::SeqCst);
        self.0.running_tasks_notify.notify_waiters();

        task.set_running(false);
    }

    async fn run_pending_periodic_tasks(&self, ticked_at: DateTime<Utc>) {
        // FIXME: It is not a good idea to have a list of tasks
        //        needed to run then sort them out with their priorities
        let mut sorted_tasks = Vec::new();
        let tasks = self.0.periodic_tasks.read().await.to_vec();
        for task in tasks.iter() {
            let should_not_run = task.is_blocked().await || task.is_running();
            if should_not_run {
                continue;
            }

            let Some(deadline) = task.deadline().await else {
                continue;
            };

            if ticked_at < deadline {
                continue;
            }

            sorted_tasks.push(task.clone());
        }

        // sort tasks needed to run based on their priority
        sorted_tasks.sort_by(|a, b| a.priority().cmp(&b.priority()));

        for task in sorted_tasks {
            let queue_tx = self.clone();
            let fut = async move { queue_tx.perform_periodic_task(ticked_at, task).await };
            self.spawn_fut(fut);
        }
    }
}

pub fn resolve_time_delta(delta: TimeDelta) -> Option<StdDuration> {
    // default is 10 seconds
    delta.to_std().or_else(|_| delta.abs().to_std()).ok()
}

// the main loop for periodic tasks
pub async fn start<S>(queue: Queue<S>)
where
    S: Clone + Send + Sync + 'static,
{
    tracing::debug!("spawned runner thread");

    // this is prevent from Eden's scheduler system to request
    // from the database multiple times. run them one at a time
    let queue_poll_duration = resolve_time_delta(queue.0.config.queue_poll_interval)
        .unwrap_or_else(|| StdDuration::from_secs(5));

    let periodic_poll_duration = resolve_time_delta(queue.0.config.periodic_poll_interval)
        .unwrap_or_else(|| StdDuration::from_millis(100));

    let mut queue_interval = tokio::time::interval(queue_poll_duration);
    let mut periodic_interval = tokio::time::interval(periodic_poll_duration);
    let queue_locked = Arc::new(AtomicBool::new(false));

    // TODO: wait a bit longer if run_pending_queue_tasks fails
    loop {
        tracing::trace!("runner loop started");

        let now = Utc::now();
        let instant = std::time::Instant::now();

        let shutdown = queue.0.shutdown.cancelled();
        tokio::select! {
            _ = periodic_interval.tick() => {
                queue.run_pending_periodic_tasks(now).await;
            },
            // this operation will take a while because this performs
            // database-related operations.
            _ = queue_interval.tick() => {
                let is_locked = queue_locked.load(Ordering::SeqCst);
                if is_locked {
                    continue;
                }
                let queue_locked_tx = queue_locked.clone();
                let queue_tx = queue.clone();
                queue.spawn_fut(async move {
                    queue_locked_tx.store(true, Ordering::SeqCst);
                    if let Err(error) = queue_tx.run_pending_queued_tasks(now).await {
                        tracing::warn!(error = %error.anonymize(), "failed to run all pending queued tasks");
                    }
                    queue_locked_tx.store(false, Ordering::SeqCst);
                });
            },
            () = shutdown => {
                tracing::debug!("requested shutdown from queue. closed runner thread...");
                return;
            },
        }

        let elapsed = instant.elapsed();
        tracing::trace!(?elapsed, "runner loop ended");
    }
}
