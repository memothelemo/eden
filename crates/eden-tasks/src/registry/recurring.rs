use chrono::{DateTime, Utc};
use eden_tasks_schema::types::TaskPriority;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::task::{Task, TaskTrigger};

pub struct RecurringTask {
    /// It should not be ran on a regular time basis.
    ///
    /// It will be if a recurring task fails and needs to be
    /// queued to the database for another execution in a
    /// later time.
    blocked: Mutex<bool>,
    deadline: Mutex<Option<DateTime<Utc>>>,
    running: AtomicBool,

    // Task configuration
    pub(crate) kind: &'static str,
    pub(crate) rust_name: &'static str,

    priority: TaskPriority,
    trigger: TaskTrigger,
}

impl RecurringTask {
    pub fn new<S, T>() -> Arc<Self>
    where
        S: Clone + Send + Sync + 'static,
        T: Task<State = S>,
    {
        Arc::new(Self {
            blocked: Mutex::new(false),
            deadline: Mutex::new(None),
            running: AtomicBool::new(false),
            kind: T::kind(),
            rust_name: std::any::type_name::<T>(),
            priority: T::priority(),
            trigger: T::trigger(),
        })
    }

    #[must_use]
    pub fn running_guard(&self) -> RecurringTaskRunningGuard<'_> {
        self.set_running(true);
        RecurringTaskRunningGuard { task: self }
    }

    pub async fn is_blocked(&self) -> bool {
        *self.blocked.lock().await
    }

    pub async fn set_blocked(&self, value: bool) {
        *self.blocked.lock().await = value;
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn set_running(&self, value: bool) {
        self.running.store(value, Ordering::SeqCst);
    }

    #[must_use]
    pub async fn deadline(&self) -> Option<DateTime<Utc>> {
        *self.deadline.lock().await
    }

    #[must_use]
    pub fn priority(&self) -> TaskPriority {
        self.priority
    }

    #[must_use]
    pub fn trigger(&self) -> &TaskTrigger {
        &self.trigger
    }
}

impl RecurringTask {
    pub async fn update_deadline(&self, now: DateTime<Utc>) {
        // blocked tasks are not allowed to adjust their own deadline yet
        if self.is_blocked().await {
            return;
        }

        let mut deadline = self.deadline.lock().await;
        *deadline = self.trigger.upcoming(Some(now));
    }
}

pub struct RecurringTaskRunningGuard<'a> {
    task: &'a RecurringTask,
}

impl<'a> Drop for RecurringTaskRunningGuard<'a> {
    fn drop(&mut self) {
        self.task.set_running(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::SampleRecurringTask;

    #[tokio::test]
    async fn update_deadline() {
        let task = RecurringTask::new::<_, SampleRecurringTask>();
        task.set_blocked(true).await;

        let now = Utc::now();
        task.update_deadline(now).await;
        assert_eq!(task.deadline().await, None);

        task.set_blocked(false).await;
        task.update_deadline(now).await;
        assert!(task.deadline().await.is_some());
    }

    #[test]
    fn running_guard() {
        let task = RecurringTask::new::<_, SampleRecurringTask>();
        let guard = task.running_guard();
        assert_eq!(task.is_running(), true);
        drop(guard);
        assert_eq!(task.is_running(), false);
    }
}
