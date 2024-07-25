use std::sync::Arc;

use dashmap::DashMap;

use super::{TaskScheduler, TaskSchedulerInternal};

#[derive(Debug, Clone)]
#[must_use = "TaskSchedulerConfig is lazy. Use `.build()` to build into TaskScheduler"]
pub struct TaskSchedulerConfig {
    pub(crate) concurrency: usize,
    pub(crate) max_failed_attempts: u32,
    pub(crate) poll_interval_secs: u64,
}

pub struct BuilderState;

impl TaskScheduler {
    pub const fn builder() -> TaskSchedulerConfig {
        TaskSchedulerConfig::new()
    }
}

impl TaskSchedulerConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            concurrency: 10,
            max_failed_attempts: 3,
            poll_interval_secs: 10,
        }
    }

    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    pub fn max_failed_attempts(mut self, max_failed_attempts: u32) -> Self {
        self.max_failed_attempts = max_failed_attempts;
        self
    }

    pub fn poll_interval_secs(mut self, poll_interval_secs: u64) -> Self {
        self.poll_interval_secs = poll_interval_secs;
        self
    }

    #[must_use]
    pub fn build<S>(self, pool: sqlx::PgPool, state: S) -> TaskScheduler<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        TaskScheduler(Arc::new(TaskSchedulerInternal {
            config: self,
            pool,
            registry: Arc::new(DashMap::new()),
            state,
        }))
    }
}
