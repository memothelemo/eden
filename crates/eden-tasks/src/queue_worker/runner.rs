use chrono::{DateTime, Utc};
use eden_tasks_schema::types::Task;
use eden_utils::sql::SqlErrorExt;
use eden_utils::Result;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, trace, warn, Instrument};

use super::task_manager::{PendingTask, QueueWorkerTaskManager};
use super::QueueWorker;

#[derive(Clone)]
pub struct QueueWorkerRunner<S> {
    errors: Arc<AtomicUsize>,
    pull_queue_block: Arc<AtomicBool>,
    should_setup_worker: Arc<AtomicBool>,
    task_manager: QueueWorkerTaskManager,
    worker: QueueWorker<S>,
}

impl<S: Clone + Send + Sync + 'static> QueueWorkerRunner<S> {
    #[must_use]
    pub fn new(worker: QueueWorker<S>, setup_later: bool) -> Self {
        Self {
            errors: Arc::new(AtomicUsize::new(0)),
            pull_queue_block: Arc::new(AtomicBool::new(true)),
            should_setup_worker: Arc::new(AtomicBool::new(setup_later)),
            task_manager: worker.0.task_manager.clone(),
            worker,
        }
    }

    #[allow(unused_assignments, unused_mut)]
    #[tracing::instrument(skip_all, fields(
        worker.id = %self.worker.id(),
        worker.concurrency = %self.worker.0.max_running_tasks,
        worker.queued_tasks_per_batch = %self.worker.0.queued_tasks_per_batch,
    ))]
    pub async fn run(mut self) {
        info!("started queue worker {}", self.worker.id());

        let mut sleep_duration = DEFAULT_INTERVAL;
        loop {
            trace!("runner loop start");

            let now = Utc::now();
            let instant = Instant::now();

            let action = self.next_action(now).await;
            trace!("received action: {action:?}");
            match action {
                RunnerAction::Continue => {
                    if sleep_duration != DEFAULT_INTERVAL {
                        info!(
                            "queue worker {} went back to healthy status",
                            self.worker.id()
                        );
                    }
                    sleep_duration = DEFAULT_INTERVAL;
                }
                RunnerAction::TimedOut => {
                    warn!("queue worker {} timed out for {TIMED_OUT_INTERVAL:?} because of these consecutive errors", self.worker.id());
                    sleep_duration = TIMED_OUT_INTERVAL;
                }
                RunnerAction::Close => {
                    debug!("closing worker runner {}", self.worker.id());
                    break;
                }
            };

            let elapsed = instant.elapsed();
            trace!(?elapsed, "runner loop ended");

            let sleep = Box::pin(tokio::time::sleep(sleep_duration));
            let closed = Box::pin(self.task_manager.closed());
            tokio::select! {
                _ = closed => {
                    info!("closing queue worker {}", self.worker.id());
                    break;
                }
                _ = sleep => {}
            }
        }
    }

    async fn next_action(&self, now: DateTime<Utc>) -> RunnerAction {
        // Setup worker if the database is healthy after some time
        let should_setup_worker = self.should_setup_worker.load(Ordering::Relaxed);
        if should_setup_worker {
            debug!("attempting to set up worker");

            // Then, try to set up as usual...
            let result = self.worker.setup().await;
            if result.is_pool_error() {
                debug!("database is unhealthy, skipping loop");

                let errors = self.errors.load(Ordering::Relaxed);
                self.errors
                    .store(errors.checked_add(1).unwrap_or_default(), Ordering::Relaxed);

                return RunnerAction::Continue;
            }

            if let Err(error) = result {
                warn!(error = %error.anonymize(), "got an error while setting up worker");
            }

            let errors = self.errors.load(Ordering::Relaxed);
            if errors > 0 {
                info!(
                    "database is healthy. ready to pull pending tasks for worker {}",
                    self.worker.id()
                );
            }
            self.errors.store(0, Ordering::Relaxed);
            self.should_setup_worker.store(false, Ordering::Relaxed);
        }

        tokio::select! {
            result = self.run_pending_tasks(now) => match result {
                Ok(..) => {}
                Err(error) => {
                    warn!(%error, "failed to run all pending tasks");

                    let errors = self.errors.fetch_add(1, Ordering::Relaxed);
                    if errors >= MAX_ERRORS_UNTIL_TIMED_OUT {
                        return RunnerAction::TimedOut;
                    }
                }
            },
            _ = self.task_manager.closed() => {
                return RunnerAction::Close;
            }
        }
        RunnerAction::Continue
    }

    #[tracing::instrument(skip_all, fields(%now), name = "loop", level = "debug")]
    async fn run_pending_tasks(&self, now: DateTime<Utc>) -> Result<()> {
        self.worker.requeue_stalled_tasks(now).await?;

        let pending_tasks = self.pull_pending_tasks(now).await?;
        if pending_tasks.len() > 0 {
            debug!("pulled {} pending task(s)", pending_tasks.len());
        } else {
            trace!("pulled {} pending task(s)", pending_tasks.len());
        }

        let pull_queue_block_tx = self.pull_queue_block.clone();
        let process_queue_tasks = self.pull_queue_block.load(Ordering::Relaxed);
        let task_manager = self.task_manager.clone();
        let worker = self.worker.clone();

        let span = tracing::Span::current();
        eden_utils::tokio::spawn(
            "eden_tasks::worker::handle_pending_tasks",
            async move {
                let mut queued_set = Vec::new();
                for task in pending_tasks {
                    let is_queued_task = !task.is_recurring();
                    let handle = task_manager.handle_pending_task(now, worker.clone(), task);
                    if is_queued_task && process_queue_tasks {
                        queued_set.push(handle);
                    }
                }

                if queued_set.is_empty() {
                    return;
                }
                pull_queue_block_tx.store(false, Ordering::Relaxed);

                debug!("waiting for batch of queued task(s) to be completed",);
                for handle in queued_set {
                    let _ = handle.await;
                }

                debug!("batch of queued tasks completed");
                pull_queue_block_tx.store(true, Ordering::Relaxed);
            }
            .instrument(span),
        );

        Ok(())
    }

    async fn pull_pending_tasks(&self, now: DateTime<Utc>) -> Result<Vec<PendingTask>> {
        let mut pending_tasks = Vec::new();
        let registry = &self.worker.0.registry;

        for task in registry.recurring_tasks().await.iter() {
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

            pending_tasks.push(PendingTask::Recurring {
                deadline,
                task: task.clone(),
            });
        }

        let can_pull_queued = self.pull_queue_block.load(Ordering::Relaxed);
        if can_pull_queued {
            trace!("fetching batch of queued tasks...");

            // not much data is lost if converted from u16 to i32
            let max_attempts = self.worker.0.max_attempts as i32;

            // wait for queued tasks to be finished before moving into
            // the next batch of tasks.
            let mut stream = Task::pull_all_pending(self.worker.id(), max_attempts, Some(now))
                .limit(self.worker.0.queued_tasks_per_batch)
                .build()
                .size(50);

            let mut conn = self.worker.db_connection().await?;
            let mut pulled_queued_tasks = 0;
            while let Some(tasks) = stream.next(&mut conn).await? {
                pulled_queued_tasks += tasks.len();
                for task in tasks {
                    pending_tasks.push(PendingTask::Queued(task));
                }
                trace!("pending_tasks.len() = {}", pending_tasks.len());
            }

            trace!("pulled batch of {pulled_queued_tasks} queued task(s)");
        } else {
            trace!("pulling queued tasks timed out");
        }

        // Sort tasks needed to run based on their priority and deadline
        pending_tasks.sort_by(|a, b| {
            a.priority()
                .cmp(&b.priority())
                .cmp(&a.deadline().cmp(&b.deadline()))
        });

        Ok(pending_tasks)
    }
}

// We need to wait for 30 seconds if one iteration fails
const TIMED_OUT_INTERVAL: Duration = Duration::from_secs(30);
const DEFAULT_INTERVAL: Duration = Duration::from_millis(100);

const MAX_ERRORS_UNTIL_TIMED_OUT: usize = 2;

#[derive(Debug)]
enum RunnerAction {
    Continue,
    TimedOut,
    Close,
}

#[allow(unused)]
#[cfg(test)]
mod tests {
    use super::QueueWorkerRunner;
    use static_assertions::assert_impl_one;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct State;

    fn test_generics<T: Sync + Send>() {}
    fn test() {
        test_generics::<QueueWorkerRunner<State>>();
    }
}
