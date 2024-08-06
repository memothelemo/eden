use chrono::TimeDelta;
use doku::Document;
use eden_tasks_schema::types::WorkerId;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::num::{NonZeroU64, NonZeroUsize};
use typed_builder::TypedBuilder;

#[serde_as]
#[derive(Debug, Deserialize, Document, Serialize, TypedBuilder)]
#[serde(default)]
pub struct Settings {
    /// Assigned queue worker ID. This field allows for the entire
    /// workers to equally distribute tasks based on their worker ID
    /// without any conflicts.
    ///
    /// It defaults to `[0, 1]` if not set.
    #[doku(as = "Vec<u32>", example = "0, 1")]
    #[builder(default = WorkerId::ONE)]
    pub(crate) id: WorkerId,

    /// Maximum amount of tasks both recurring and queued running
    /// at the same time. If one task needs to perform, it has to
    /// wait until a running task before the queue filled up,
    /// completes their operation.
    ///
    /// It defaults to `10` if not set.
    #[doku(as = "usize", example = "10")]
    #[builder(default = NonZeroUsize::new(10).unwrap())]
    pub(crate) max_running_tasks: NonZeroUsize,

    /// Amount of retries that will make a task give up or cancel if
    /// it exceeds the limit.
    ///
    /// It defaults to `3` retries if not set.
    #[doku(as = "u16", example = "3")]
    #[builder(default = 3)]
    pub(crate) max_task_retries: u16,

    /// Processes a specified number of queued tasks in a batch and waits
    /// for all them to complete before proceeding to another batch of
    /// queued tasks.
    ///
    /// It defaults to `50` if not set.
    #[doku(as = "u64", example = "50")]
    #[builder(default = NonZeroU64::new(50).unwrap())]
    pub(crate) queued_tasks_per_batch: NonZeroU64,

    /// The minimum duration threshold will consider running queued
    /// tasks stalled and must be requeued again.
    ///
    /// It defaults to `30 minutes` if not set.
    #[doku(as = "String", example = "30m")]
    #[serde_as(as = "eden_utils::serial::AsHumanDuration")]
    #[builder(default = TimeDelta::minutes(30))]
    pub(crate) stalled_tasks_threshold: TimeDelta,
}

impl Settings {
    #[must_use]
    pub fn id(self) -> WorkerId {
        self.id
    }

    #[must_use]
    pub fn max_running_tasks(&self) -> usize {
        self.max_running_tasks.get()
    }

    #[must_use]
    pub fn max_task_retries(&self) -> u16 {
        self.max_task_retries
    }

    #[must_use]
    pub fn stalled_tasks_threshold(&self) -> TimeDelta {
        self.stalled_tasks_threshold
    }
}

impl Default for Settings {
    #[allow(clippy::unwrap_used)]
    fn default() -> Self {
        Self {
            id: WorkerId::ONE,
            max_running_tasks: NonZeroUsize::new(10).unwrap(),
            max_task_retries: 3,
            queued_tasks_per_batch: NonZeroU64::new(50).unwrap(),
            stalled_tasks_threshold: TimeDelta::minutes(30),
        }
    }
}
