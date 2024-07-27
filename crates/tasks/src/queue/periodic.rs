use chrono::{DateTime, Utc};
use eden_db::schema::TaskPriority;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

use crate::{Task, TaskSchedule};

pub struct PeriodicTask {
    // periodic task should not be ran if it is set to true
    // (maybe because it is already queued in the database)
    blocked: Mutex<bool>,
    running: AtomicBool,

    // task parameters
    priority: TaskPriority,
    schedule: TaskSchedule,

    // task running status
    deadline: Mutex<Option<DateTime<Utc>>>,

    pub task_type: &'static str,
}

impl PeriodicTask {
    #[must_use]
    pub fn new<S, T>() -> Self
    where
        T: Task<State = S>,
        S: Clone + Send + Sync + 'static,
    {
        Self {
            blocked: Mutex::new(false),
            running: AtomicBool::new(false),

            priority: T::priority(),
            schedule: T::schedule(),

            deadline: Mutex::new(None),

            task_type: T::task_type(),
        }
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
    pub fn schedule(&self) -> &TaskSchedule {
        &self.schedule
    }
}

impl PeriodicTask {
    pub async fn adjust_deadline(&self, now: DateTime<Utc>) {
        // blocked tasks are not allowed to adjust their own deadline yet
        if self.is_blocked().await {
            return;
        }

        let mut deadline = self.deadline.lock().await;
        *deadline = self.schedule.upcoming(Some(now));
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn set_running(&self, value: bool) {
        self.running.store(value, Ordering::SeqCst);
    }

    pub async fn is_blocked(&self) -> bool {
        *self.blocked.lock().await
    }

    pub async fn set_blocked(&self, value: bool) {
        *self.blocked.lock().await = value;
    }
}
