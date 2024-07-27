use super::{Queue, QueueConfig, QueueInner};
use dashmap::DashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, RwLock, Semaphore};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

pub struct BuilderState;

impl Queue<BuilderState> {
    pub const fn builder() -> QueueConfig {
        QueueConfig::new()
    }
}

impl QueueConfig {
    pub fn build<S>(self, pool: sqlx::PgPool, state: S) -> Queue<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        let semaphore = Semaphore::new(self.concurrency);
        Queue(Arc::new(QueueInner {
            config: self,
            periodic_tasks: RwLock::new(Vec::new()),

            pool,
            registry: DashMap::new(),
            state,

            future_tracker: TaskTracker::new(),
            runner_handle: Mutex::new(None),

            running_tasks: AtomicUsize::new(0),
            running_tasks_notify: Notify::new(),

            semaphore,
            shutdown: CancellationToken::new(),
        }))
    }
}
