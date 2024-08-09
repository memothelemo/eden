use chrono::{DateTime, Utc};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::types::{TaskPriority, TaskRawData, TaskStatus};

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertTaskForm {
    #[builder(default)]
    pub id: Option<Uuid>,
    pub data: TaskRawData,
    pub deadline: DateTime<Utc>,
    #[builder(default)]
    pub attempts: i32,
    #[builder(default)]
    pub periodic: bool,
    #[builder(default)]
    pub priority: TaskPriority,
    #[builder(default)]
    pub status: TaskStatus,
}

#[derive(Debug, Clone, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct UpdateTaskForm {
    pub attempts: Option<i32>,
    pub data: Option<TaskRawData>,
    pub deadline: Option<DateTime<Utc>>,
    pub last_retry: Option<DateTime<Utc>>,
    pub priority: Option<TaskPriority>,
    pub status: Option<TaskStatus>,
}
