use chrono::TimeDelta;
use doku::Document;
use fancy_duration::AsFancyDuration;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt::Debug;

#[serde_as]
#[derive(Debug, Deserialize, Document, Serialize)]
#[serde(default)]
pub struct Queue {
    /// Maximum amount of running tasks running at the same.
    ///
    /// It defaults to `10` if not set.
    #[doku(example = "10")]
    pub(crate) max_running_tasks: usize,

    /// How many retries that make the task give up if it
    /// exceeds the limit.
    ///
    /// It defaults to `3` retries if not set.
    #[doku(example = "3")]
    pub(crate) max_task_retries: u16,

    /// Parameters for controlling the polling in Eden's
    /// task queueing system.
    ///
    /// **Do not modify if you don't know how Eden's queueing system works!**
    pub(crate) polling: QueuePolling,

    /// The minimum duration threshold will consider running queued
    /// tasks stalled and must be requeued again.
    ///
    /// It defaults to `30` minutes if not set.
    #[doku(as = "String", example = "30m")]
    #[serde_as(as = "eden_utils::serial::AsHumanDuration")]
    pub(crate) stalled_tasks_threshold: TimeDelta,
}

#[serde_as]
#[derive(Document, Deserialize, Serialize)]
#[serde(default)]
pub struct QueuePolling {
    /// The duration to tick and regularly check and run all
    /// pending periodic tasks.
    ///
    /// It defaults to `100` milliseconds if not set.
    #[doku(as = "String", example = "100ms")]
    #[serde_as(as = "eden_utils::serial::AsHumanDuration")]
    pub(crate) periodic: TimeDelta,
    /// The duration to tick and regularly check and run all
    /// pending queued tasks (tasks stored in the database).
    ///
    /// It defaults to `5` seconds if not set.
    #[doku(as = "String", example = "5s")]
    #[serde_as(as = "eden_utils::serial::AsHumanDuration")]
    pub(crate) queue: TimeDelta,
}

impl Queue {
    #[must_use]
    pub fn max_running_tasks(&self) -> usize {
        self.max_running_tasks
    }

    #[must_use]
    pub fn max_task_retries(&self) -> u16 {
        self.max_task_retries
    }

    #[must_use]
    pub fn polling(&self) -> &QueuePolling {
        &self.polling
    }
}

impl QueuePolling {
    #[must_use]
    pub fn periodic(&self) -> TimeDelta {
        self.periodic
    }

    #[must_use]
    pub fn queue(&self) -> TimeDelta {
        self.queue
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self {
            max_running_tasks: 10,
            max_task_retries: 3,
            polling: QueuePolling::default(),
            stalled_tasks_threshold: TimeDelta::minutes(30),
        }
    }
}

impl Default for QueuePolling {
    fn default() -> Self {
        Self {
            periodic: TimeDelta::milliseconds(100),
            queue: TimeDelta::seconds(5),
        }
    }
}

impl Debug for QueuePolling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueuePolling")
            .field("periodic", &self.periodic.fancy_duration().format())
            .field("queue", &self.queue.fancy_duration().format())
            .finish()
    }
}
