use chrono::{DateTime, Utc};
use eden_utils::error::AnyResultExt;
use eden_utils::{error::ResultExt, Result};
use uuid::Uuid;

use crate::forms::{InsertJobForm, UpdateJobForm};
use crate::paged_queries::{GetAllJobs, PullAllPendingJobs};
use crate::schema::{Job, JobStatus};
use crate::utils::Paginated;
use crate::QueryError;

impl Job {
    pub async fn fail(conn: &mut sqlx::PgConnection, id: Uuid) -> Result<Self, QueryError> {
        sqlx::query_as::<_, Self>(
            r"UPDATE jobs
            SET status = $1,
                failed_attempts = failed_attempts + 1
            WHERE id = $2",
        )
        .bind(JobStatus::Failed)
        .bind(id)
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not fail job from id")
    }

    pub async fn from_id(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as(r"SELECT * FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not get job from id")
    }

    pub fn get_all() -> Paginated<GetAllJobs> {
        Paginated::new(GetAllJobs)
    }

    pub fn pull_all_pending(
        max_failed_attempts: i64,
        now: Option<DateTime<Utc>>,
    ) -> Paginated<PullAllPendingJobs> {
        Paginated::new(PullAllPendingJobs {
            max_failed_attempts,
            now: now.unwrap_or_else(Utc::now),
        })
    }
}

impl Job {
    pub async fn insert(
        conn: &mut sqlx::PgConnection,
        form: InsertJobForm,
    ) -> Result<Self, QueryError> {
        // It has to be serialized before giving it to the database
        let data = serde_json::to_value(&form.data)
            .anonymize_error()
            .transform_context(QueryError)
            .attach_printable("could not serialize task to insert job")?;

        sqlx::query_as::<_, Job>(
            r"INSERT INTO jobs (deadline, priority, status, data)
            VALUES ($1, $2, $3, $4)
            RETURNING *",
        )
        .bind(form.deadline)
        .bind(form.priority)
        .bind(form.status)
        .bind(data)
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not insert job")
    }

    pub async fn update(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        form: UpdateJobForm,
    ) -> Result<Option<Self>, QueryError> {
        // sqlx treated serde_json::Value value as jsonb type
        let data = match form.data {
            Some(n) => Some(
                serde_json::to_value(&n)
                    .anonymize_error()
                    .transform_context(QueryError)
                    .attach_printable("could not serialize task to insert job")?,
            ),
            None => None,
        };

        sqlx::query_as::<_, Job>(
            r"UPDATE jobs
            SET deadline = COALESCE($1, deadline),
                failed_attempts = COALESCE($2, failed_attempts),
                last_retry = COALESCE($3, last_retry),
                priority = COALESCE($4, priority),
                status = COALESCE($5, status),
                data = COALESCE($6, data),
                updated_at = $7
            WHERE id = $8
            RETURNING *",
        )
        .bind(form.deadline)
        .bind(form.failed_attempts)
        .bind(form.last_retry)
        .bind(form.priority)
        .bind(form.status)
        .bind(data)
        // due to limitations with PullAllQueueJobs query, we have to
        // bind this argument to update `updated_at` manually.
        .bind(Utc::now())
        .bind(id)
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not update job from id")
    }

    pub async fn delete(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Job>(r"DELETE FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not delete job from id")
    }

    pub async fn delete_all(conn: &mut sqlx::PgConnection) -> Result<u64, QueryError> {
        sqlx::query(r"DELETE FROM jobs")
            .execute(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not delete all jobs")
            .map(|v| v.rows_affected())
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use crate::schema::{JobPriority, JobRawData, JobStatus};
    use crate::test_utils;

    use super::*;
    use chrono::Utc;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let job = test_utils::generate_job(&mut conn).await?;

        assert!(Job::from_id(&mut conn, job.id)
            .await
            .anonymize_error()?
            .is_some());

        Job::delete(&mut conn, job.id).await.anonymize_error()?;

        assert!(Job::from_id(&mut conn, job.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let deadline = Utc::now();
        let data = JobRawData {
            kind: "foo".into(),
            data: serde_json::json!({
                "currency": "PHP",
                "deadline": Utc::now(),
                "payer_id": "613425648685547541",
                "price": 15.0,
            }),
        };

        let form = InsertJobForm::builder()
            .deadline(deadline)
            .priority(JobPriority::High)
            .data(data.clone())
            .build();

        // milisecond precision lost for this: assert_eq!(job.deadline, deadline);
        let job = Job::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(job.failed_attempts, 0);
        assert_eq!(job.priority, JobPriority::High);
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.data, data);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_update(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let job = test_utils::generate_job(&mut conn).await?;

        let new_deadline = Utc::now();
        let form = UpdateJobForm::builder()
            .deadline(Some(new_deadline))
            .failed_attempts(Some(2))
            .priority(Some(JobPriority::Low))
            .status(Some(JobStatus::Failed))
            .build();

        let new_data = Job::update(&mut conn, job.id, form)
            .await
            .anonymize_error()?;

        assert!(new_data.is_some());

        // milisecond precision lost for this: assert_eq!(new_data.deadline, new_deadline);
        let new_data = new_data.unwrap();
        assert!(new_data.updated_at.is_some());
        assert_eq!(new_data.failed_attempts, 2);
        assert_eq!(new_data.priority, JobPriority::Low);
        assert_eq!(new_data.status, JobStatus::Failed);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_delete(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let job = test_utils::generate_job(&mut conn).await?;

        assert!(Job::from_id(&mut conn, job.id)
            .await
            .anonymize_error()?
            .is_some());

        Job::delete(&mut conn, job.id).await.anonymize_error()?;

        assert!(Job::from_id(&mut conn, job.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }
}
