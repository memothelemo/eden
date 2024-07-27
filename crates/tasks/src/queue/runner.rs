use chrono::{DateTime, TimeDelta, Utc};
use eden_db::forms::UpdateTaskForm;
use eden_db::schema::{Task, TaskRawData, TaskStatus};
use eden_utils::error::{AnyResultExt, ErrorExt, ResultExt};
use eden_utils::Result;
use futures::FutureExt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration as StdDuration;
use thiserror::Error;
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
    async fn perform_queued_task(&self, task_info: &Task) -> PerformTaskAction {
        let Some(registry_meta) = self.0.registry.get(task_info.data.kind.as_str()) else {
            eprintln!(
                "Cannot find task registry metadata for {:?}, skipping...",
                task_info.data.kind
            );
            return PerformTaskAction::Delete;
        };

        let perform_info = TaskPerformInfo {
            id: task_info.id,
            created_at: task_info.created_at,
            attempts: task_info.attempts,
            last_retry: task_info.last_retry,
            is_retrying: task_info.attempts > 0,
        };

        let task = match self.deserialize_task(&task_info.data, &registry_meta) {
            Ok(n) => n,
            Err(error) => {
                eprintln!(
                    "could not deserialize task ({:?}) {:#?}",
                    task_info.data.kind,
                    error.anonymize()
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
                eprintln!(
                    "task {} with type {:?}; task failed: {error:#?}",
                    perform_info.id, registry_meta.kind,
                );

                error
                    .get_attached_any()
                    .next()
                    .cloned()
                    .unwrap_or(PerformTaskAction::RetryOnError)
            }
        };

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
        println!("requeued {stalled_tasks} stalled tasks");

        let mut stream = Task::pull_all_pending(max_attempts, Some(ticked_at)).size(50);
        while let Some(tasks) = stream
            .next(&mut conn)
            .await
            .change_context(RunPendingQueuedTaskError)?
        {
            for task in tasks {
                let queue_tx = self.clone();
                self.spawn_fut(async move {
                    let action = queue_tx.perform_queued_task(&task).await;

                    let mut conn = match queue_tx.db_connection().await {
                        Ok(conn) => conn,
                        Err(error) => {
                            eprintln!(
                                "task {} with type {:?}; could not establish database connection: {error:#?}",
                                task.id, task.data.kind
                            );
                            return;
                        }
                    };

                    let now = Utc::now();
                    let result = match action {
                        PerformTaskAction::Delete => {
                            eprintln!(
                                "task {} with type {:?}; deleting task",
                                task.id, task.data.kind
                            );
                            Task::delete(&mut conn, task.id).await.map(|_| ())
                                .anonymize_error()
                        },
                        PerformTaskAction::Completed => {
                            eprintln!(
                                "task {} with type {:?}; task completed",
                                task.id, task.data.kind
                            );

                            let form = UpdateTaskForm::builder()
                                .status(Some(TaskStatus::Success))
                                .build();

                            Task::update(&mut conn, task.id, form).await.map(|_| ())
                                .anonymize_error()
                        },
                        PerformTaskAction::RetryIn(delta) => {
                            let total_attempts = task.attempts + 1;
                            if total_attempts > max_attempts {
                                eprintln!(
                                    "task {} with type {:?}; task failed, exceeded maximum of retries",
                                    task.id, task.data.kind
                                );
                                Task::fail(&mut conn, task.id).await.map(|_| ())
                                    .anonymize_error()
                            } else {
                                eprintln!(
                                    "task {} with type {:?}; retrying task in {delta}",
                                    task.id, task.data.kind
                                );
                                queue_tx.requeue(task.id, Scheduled::In(delta), Some(now), task.attempts + 1)
                                    .await
                                    .anonymize_error()
                            }
                        },
                        PerformTaskAction::RetryOnError | PerformTaskAction::RetryOnTimedOut => unreachable!(),
                    };

                    if let Err(error) = result {
                        eprintln!(
                            "task {} with type {:?}; failed to perform post-operation task: {error}",
                            task.id, task.data.kind
                        );
                        return;
                    }

                    // Unblock if it is a periodic task
                    if let Some(periodic_task) = queue_tx.get_periodic_task(&task.data.kind).await {
                        eprintln!(
                            "periodic task {} with type {:?}; unblocking task",
                            task.id, task.data.kind
                        );
                        periodic_task.set_blocked(false).await;
                    }
                });
            }
        }

        Ok(())
    }
}

impl<S> Queue<S>
where
    S: Clone + Send + Sync + 'static,
{
    async fn perform_periodic_task(&self, ticked_at: DateTime<Utc>, task: &PeriodicTask) {
        let Some(registry_meta) = self.0.registry.get(task.task_type) else {
            panic!(
                "Cannot find task registry metadata for {:?}",
                task.task_type
            );
        };
        task.set_running(true);

        // Generate a fake queued job data
        let fake_queued_task_id = Uuid::new_v4();
        let perform_info = TaskPerformInfo {
            id: fake_queued_task_id,
            created_at: ticked_at,
            attempts: 0,
            last_retry: None,
            is_retrying: false,
        };

        let raw_data = TaskRawData {
            kind: task.task_type.into(),
            inner: serde_json::Value::Null,
        };

        let deserialized_task = match self.deserialize_task(&raw_data, &registry_meta) {
            Ok(n) => n,
            Err(error) => {
                let error = error
                    .attach_printable("suggestion: do not put any data needed for periodic tasks")
                    .anonymize();

                panic!(
                    "could not deserialize task ({:?}) {error:#?}",
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
                eprintln!(
                    "task {} with type {:?}; task failed: {error:#?}",
                    perform_info.id, registry_meta.kind,
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
            eprintln!(
                "task {} with type {:?}; could not queue periodic task: {:#?}",
                perform_info.id,
                registry_meta.kind,
                error.anonymize()
            );
        } else {
            eprintln!(
                "task {} with type {:?}; queued for {retry_in}",
                perform_info.id, registry_meta.kind,
            );
            task.set_blocked(true).await;
            task.set_running(false);
        }
    }

    async fn run_pending_periodic_tasks(&self, ticked_at: DateTime<Utc>) {
        // FIXME: It is not a good idea to have a list of tasks
        //        needed to run then sort them out with their priorities
        let mut unsorted_futures = Vec::new();
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

            let queue_tx = self.clone();
            let task_tx = task.clone();
            let future =
                async move { queue_tx.perform_periodic_task(ticked_at, &task_tx).await }.boxed();

            unsorted_futures.push((task.priority(), future));
        }

        // sort tasks needed to run based on their priority
        unsorted_futures.sort_by(|a, b| a.0.cmp(&b.0));
        for entry in unsorted_futures {
            self.spawn_fut(entry.1);
        }
    }
}

pub fn resolve_time_delta(delta: TimeDelta) -> Option<StdDuration> {
    // default is 10 seconds
    delta.to_std().or_else(|_| delta.abs().to_std()).ok()
}

// the main loop for queue
pub async fn runner<S>(queue: Queue<S>)
where
    S: Clone + Send + Sync + 'static,
{
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
        let now = Utc::now();
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
                        eprintln!("running all pending queued tasks failed: {}", error.anonymize());
                    }
                    queue_locked_tx.store(false, Ordering::SeqCst);
                });
            },
            () = shutdown => {
                eprintln!("requested shutdown");
                return;
            },
        }
    }
}
