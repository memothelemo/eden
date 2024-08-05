use chrono::{DateTime, Utc};
use eden_tasks_schema::types::Task;
use eden_utils::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, trace, warn, Instrument};

use super::task_manager::{PendingTask, WorkerTaskManager};
use super::Worker;

#[derive(Clone)]
pub struct WorkerRunner<S> {
    errors: usize,
    pull_queue_block: Arc<AtomicBool>,
    task_manager: WorkerTaskManager,
    worker: Worker<S>,
}

impl<S: Clone + Send + Sync + 'static> WorkerRunner<S> {
    #[must_use]
    pub fn new(worker: Worker<S>) -> Self {
        Self {
            errors: 0,
            pull_queue_block: Arc::new(AtomicBool::new(true)),
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
        info!("started worker {}", self.worker.id());

        let mut sleep_duration = DEFAULT_INTERVAL;
        loop {
            trace!("runner loop start");

            let now = Utc::now();
            let instant = Instant::now();
            match self.next_action(now).await {
                RunnerAction::Continue => {
                    sleep_duration = DEFAULT_INTERVAL;
                }
                RunnerAction::TimedOut => {
                    warn!("worker runner timed out for {TIMED_OUT_INTERVAL:?}");
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
                    debug!("closing worker runner {}", self.worker.id());
                    break;
                }
                _ = sleep => {}
            }
        }
    }

    async fn next_action(&mut self, now: DateTime<Utc>) -> RunnerAction {
        tokio::select! {
            result = self.run_pending_tasks(now) => match result {
                Ok(..) => {}
                Err(error) => {
                    warn!(%error, "failed to run all pending tasks");
                    self.errors = self
                        .errors
                        .clamp(self.errors + 1, MAX_ERRORS_UNTIL_TIMED_OUT);

                    if self.errors > MAX_ERRORS_UNTIL_TIMED_OUT {
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

            trace!("pulled {pulled_queued_tasks} queued task(s)");
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

const MAX_ERRORS_UNTIL_TIMED_OUT: usize = 3;

enum RunnerAction {
    Continue,
    TimedOut,
    Close,
}
