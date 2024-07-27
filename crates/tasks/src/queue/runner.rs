use chrono::{DateTime, TimeDelta, Utc};
use eden_db::schema::TaskRawData;
use eden_utils::error::ErrorExt;
use futures::FutureExt;
use std::time::Duration as StdDuration;
use uuid::Uuid;

use crate::{Scheduled, TaskPerformInfo};

use super::{periodic::PeriodicTask, PerformTaskAction, Queue};

impl<S> Queue<S>
where
    S: Clone + Send + Sync + 'static,
{
    async fn perform_periodic_task(&self, requested_at: DateTime<Utc>, task: &PeriodicTask) {
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
            created_at: requested_at,
            failed_attempts: 0,
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
            Err(error) => error
                .get_attached()
                .next()
                .cloned()
                .unwrap_or(PerformTaskAction::RetryOnError),
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

    async fn run_pending_periodic_tasks(&self, now: DateTime<Utc>) {
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

            if now < deadline {
                continue;
            }

            let queue_tx = self.clone();
            let task_tx = task.clone();
            let future = async move { queue_tx.perform_periodic_task(now, &task_tx).await }.boxed();
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
    let poll_interval = resolve_time_delta(queue.0.config.poll_interval)
        .unwrap_or_else(|| StdDuration::from_secs(10));

    loop {
        let now = Utc::now();
        queue.run_pending_periodic_tasks(now).await;

        let poll = tokio::time::sleep(poll_interval);
        let shutdown = queue.0.shutdown.cancelled();

        tokio::select! {
            () = poll => {},
            () = shutdown => {
                eprintln!("requested shutdown");
                return;
            },
        }
    }
}
