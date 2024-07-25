use dashmap::DashMap;
use std::sync::Arc;

use crate::JobRunner;

use super::JobRunnerInner;

#[derive(Debug, Clone)]
#[must_use = "JobRunnerConfig is lazy. Use `.build()` to build into JobRunner"]
pub struct JobRunnerConfig {
    pub(crate) concurrency: usize,
    pub(crate) max_failed_attempts: u32,
    pub(crate) poll_interval_secs: u64,
}

impl Default for JobRunnerConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl JobRunnerConfig {
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

    pub fn build<S>(self, pool: sqlx::PgPool, state: S) -> JobRunner<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        JobRunner(Arc::new(JobRunnerInner {
            config: self,
            registry: Arc::new(DashMap::new()),
            pool,
            state,
        }))
    }
}
