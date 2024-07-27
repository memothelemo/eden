use chrono::TimeDelta;

#[derive(Debug, Clone)]
#[must_use = "Queue config is lazy. Use `.build()` to build into Queue"]
pub struct QueueConfig {
    pub(crate) concurrency: usize,
    pub(crate) max_attempts: u16,

    pub(crate) periodic_poll_interval: TimeDelta,
    pub(crate) queue_poll_interval: TimeDelta,
    pub(crate) stalled_tasks_threshold: TimeDelta,
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
            max_attempts: 3,

            periodic_poll_interval: TimeDelta::milliseconds(100),
            queue_poll_interval: TimeDelta::seconds(5),
            stalled_tasks_threshold: TimeDelta::minutes(30),
        }
    }

    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    pub fn max_attempts(mut self, max_attempts: u16) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    pub fn periodic_poll_interval(mut self, poll_interval: TimeDelta) -> Self {
        self.periodic_poll_interval = poll_interval;
        self
    }

    pub fn queue_poll_interval(mut self, queue_poll_interval: TimeDelta) -> Self {
        self.queue_poll_interval = queue_poll_interval;
        self
    }

    pub fn stalled_tasks_threshold(mut self, stalled_tasks_threshold: TimeDelta) -> Self {
        self.stalled_tasks_threshold = stalled_tasks_threshold;
        self
    }
}
