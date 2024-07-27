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
    pub attempts: i32,
    pub data: TaskRawData,
    pub deadline: DateTime<Utc>,
    pub last_retry: Option<DateTime<Utc>>,
    pub periodic: bool,
    pub priority: TaskPriority,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TaskRawData {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "data")]
    pub inner: Json,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Task {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id = row.try_get("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;
        let attempts = row.try_get("attempts")?;
        let data = row.try_get::<sqlx::types::Json<TaskRawData>, _>("data")?;
        let deadline = row.try_get::<NaiveDateTime, _>("deadline")?;
        let last_retry = row.try_get::<Option<NaiveDateTime>, _>("last_retry")?;
        let periodic = row.try_get("periodic")?;
        let priority = row.try_get("priority")?;
        let status = row.try_get("status")?;

        Ok(Self {
            id,
            created_at: naive_to_dt(created_at),
            updated_at: updated_at.map(naive_to_dt),
            data: data.0,
            deadline: naive_to_dt(deadline),
            attempts,
            last_retry: last_retry.map(naive_to_dt),
            periodic,
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

#[cfg(test)]
mod tests {
    use super::TaskPriority;

    #[test]
    fn test_task_priority_order() {
        assert!(TaskPriority::High > TaskPriority::Low);
        assert!(TaskPriority::Medium > TaskPriority::Low);
    }
}
