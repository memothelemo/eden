use chrono::{DateTime, Utc};
use typed_builder::TypedBuilder;

use crate::schema::{JobPriority, JobStatus, JobTask};

#[derive(Debug, Clone, TypedBuilder)]
pub struct InsertJobForm<'a> {
    pub name: &'a str,
    pub deadline: DateTime<Utc>,
    #[builder(default)]
    pub priority: JobPriority,
    #[builder(default)]
    pub status: JobStatus,
    pub task: JobTask,
}

#[derive(Debug, Clone, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct UpdateJobForm {
    pub deadline: Option<DateTime<Utc>>,
    pub failed_attempts: Option<i64>,
    pub last_retry: Option<DateTime<Utc>>,
    pub priority: Option<JobPriority>,
    pub status: Option<JobStatus>,
}
