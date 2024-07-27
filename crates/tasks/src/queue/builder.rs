use super::{Queue, QueueConfig, QueueInner};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

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
        Queue(Arc::new(QueueInner {
            config: self,
            periodic_tasks: Arc::new(RwLock::new(Vec::new())),
            pool,
            registry: Arc::new(DashMap::new()),
            runner_handle: Arc::new(Mutex::new(None)),
            running_tasks: TaskTracker::new(),
            shutdown: CancellationToken::new(),
            state,
        }))
    }
}
