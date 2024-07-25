use chrono::{DateTime, Utc};
use typed_builder::TypedBuilder;

use crate::schema::{JobPriority, JobRawData, JobStatus};

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertJobForm {
    pub data: JobRawData,
    pub deadline: DateTime<Utc>,
    #[builder(default)]
    pub priority: JobPriority,
    #[builder(default)]
    pub status: JobStatus,
}

#[derive(Debug, Clone, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct UpdateJobForm {
    pub data: Option<JobRawData>,
    pub deadline: Option<DateTime<Utc>>,
    pub failed_attempts: Option<i64>,
    pub last_retry: Option<DateTime<Utc>>,
    pub priority: Option<JobPriority>,
    pub status: Option<JobStatus>,
}
