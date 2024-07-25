use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use sqlx::Row;
use uuid::Uuid;

use crate::utils::naive_to_dt;

#[derive(Debug, Clone)]
pub struct Task {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub data: TaskRawData,
    pub deadline: DateTime<Utc>,
    pub failed_attempts: i32,
    pub last_retry: Option<DateTime<Utc>>,
    pub priority: TaskPriority,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TaskRawData {
    #[serde(rename = "type")]
    pub kind: String,
    pub data: Json,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Task {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id = row.try_get("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;
        let data = row.try_get::<sqlx::types::Json<TaskRawData>, _>("data")?;
        let deadline = row.try_get::<NaiveDateTime, _>("deadline")?;
        let failed_attempts = row.try_get("failed_attempts")?;
        let last_retry = row.try_get::<Option<NaiveDateTime>, _>("last_retry")?;
        let priority = row.try_get("priority")?;
        let status = row.try_get("status")?;

        Ok(Self {
            id,
            created_at: naive_to_dt(created_at),
            updated_at: updated_at.map(naive_to_dt),
            data: data.0,
            deadline: naive_to_dt(deadline),
            failed_attempts,
            last_retry: last_retry.map(naive_to_dt),
            priority,
            status,
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type)]
#[sqlx(type_name = "task_priority", rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "task_status", rename_all = "lowercase")]
pub enum TaskStatus {
    Failed,
    Running,
    Success,
    #[default]
    Queued,
}
