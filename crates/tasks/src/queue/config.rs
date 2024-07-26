use chrono::TimeDelta;

#[derive(Debug, Clone)]
#[must_use = "Queue config is lazy. Use `.build()` to build into Queue"]
pub struct QueueConfig {
    pub(crate) concurrency: usize,
    pub(crate) max_failed_attempts: u32,
    pub(crate) poll_interval: TimeDelta,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueConfig {
    pub const fn new() -> Self {
        Self {
            concurrency: 10,
            max_failed_attempts: 3,
            poll_interval: TimeDelta::seconds(10),
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

    pub fn poll_interval(mut self, poll_interval: TimeDelta) -> Self {
        self.poll_interval = poll_interval;
        self
    }
}
