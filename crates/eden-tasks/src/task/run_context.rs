use chrono::{DateTime, Utc};
use eden_tasks_schema::types::{Task, WorkerId};
use uuid::Uuid;

/// It contains contextual information of a running task.
#[derive(Debug, PartialEq, Eq)]
pub struct TaskRunContext {
    pub id: Uuid,
    pub worker_id: WorkerId,
    pub created_at: DateTime<Utc>,
    pub deadline: DateTime<Utc>,
    pub attempts: i32,
    pub last_retry: Option<DateTime<Utc>>,
    pub is_retrying: bool,
}

impl TaskRunContext {
    pub(crate) fn from_recurring(
        worker_id: WorkerId,
        deadline: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            worker_id,
            created_at: now,
            deadline,
            attempts: 0,
            last_retry: None,
            is_retrying: false,
        }
    }

    pub(crate) fn from_task_schema(worker_id: WorkerId, data: &Task) -> Self {
        Self {
            id: data.id,
            worker_id,
            created_at: data.created_at,
            deadline: data.deadline,
            attempts: data.attempts,
            last_retry: data.last_retry,
            is_retrying: data.attempts > 0,
        }
    }
}
