use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

use crate::utils::naive_to_dt;

#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub updated_at: Option<DateTime<Utc>>,
    pub deadline: DateTime<Utc>,
    pub failed_attempts: i32,
    pub last_retry: Option<DateTime<Utc>>,
    pub priority: JobPriority,
    pub status: JobStatus,
    pub data: JobData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JobData {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub inner: HashMap<String, Json>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Job {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id = row.try_get("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let name = row.try_get("name")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;
        let deadline = row.try_get::<NaiveDateTime, _>("deadline")?;
        let failed_attempts = row.try_get("failed_attempts")?;
        let last_retry = row.try_get::<Option<NaiveDateTime>, _>("last_retry")?;
        let priority = row.try_get("priority")?;
        let status = row.try_get("status")?;
        // sqlx treated serde_json::Value value as jsonb type
        let data = row.try_get::<sqlx::types::Json<JobData>, _>("data")?;

        Ok(Self {
            id,
            created_at: naive_to_dt(created_at),
            name,
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

// #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
// #[serde(tag = "type", rename_all = "snake_case")]
// pub enum JobTask {
//     // This is applicable for special payments
//     BillPayer {
//         currency: String,
//         deadline: DateTime<Utc>,
//         payer_id: Id<UserMarker>,
//         price: Decimal,
//     },
// }

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type)]
#[sqlx(type_name = "job_priority", rename_all = "lowercase")]
pub enum JobPriority {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "job_status", rename_all = "lowercase")]
pub enum JobStatus {
    Failed,
    Running,
    Success,
    #[default]
    Queued,
}
