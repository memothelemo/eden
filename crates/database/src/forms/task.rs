use chrono::{DateTime, Utc};
use typed_builder::TypedBuilder;

use crate::schema::{TaskPriority, TaskRawData, TaskStatus};

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertTaskForm {
    pub data: TaskRawData,
    pub deadline: DateTime<Utc>,
    #[builder(default)]
    pub priority: TaskPriority,
    #[builder(default)]
    pub status: TaskStatus,
}

#[derive(Debug, Clone, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct UpdateTaskForm {
    pub data: Option<TaskRawData>,
    pub deadline: Option<DateTime<Utc>>,
    pub failed_attempts: Option<i64>,
    pub last_retry: Option<DateTime<Utc>>,
    pub priority: Option<TaskPriority>,
    pub status: Option<TaskStatus>,
}