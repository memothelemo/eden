use chrono::TimeDelta;
use eden_tasks_schema::types::WorkerId;
use std::fmt::Debug;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::task_manager::QueueWorkerTaskManager;
use super::QueueWorker;
use crate::registry::TaskRegistry;

pub struct QueueWorkerInner<S> {
    pub id: WorkerId,
    pub registry: TaskRegistry<S>,

    // state
    pub pool: sqlx::PgPool,
    pub runner_handle: Mutex<Option<JoinHandle<()>>>,
    pub state: S,
    pub task_manager: QueueWorkerTaskManager,

    // configuration
    pub max_attempts: u16,
    pub max_running_tasks: usize,
    pub queued_tasks_per_batch: u64,
    pub stalled_tasks_threshold: TimeDelta,
}

impl<S: Clone + Send + Sync + 'static> Debug for QueueWorkerInner<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Worker")
            .field("id", &self.id)
            .field("registry", &self.registry)
            .field("pool", &self.pool)
            .field("max_attempts", &self.max_attempts)
            .field("max_running_tasks", &self.max_running_tasks)
            .field("stalled_tasks_threshold", &self.stalled_tasks_threshold)
            .finish()
    }
}

impl<S: Clone + Send + Sync + 'static> Debug for QueueWorker<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
