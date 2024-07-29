use chrono::{DateTime, TimeDelta, Utc};
use eden_utils::error::AnyResultExt;
use eden_utils::{error::ResultExt, Result};
use uuid::Uuid;

use crate::forms::{InsertTaskForm, UpdateTaskForm};
use crate::paged_queries::{GetAllTasks, PullAllPendingTasks};
use crate::schema::{Task, TaskStatus};
use crate::utils::Paginated;
use crate::QueryError;

impl Task {
    pub async fn fail(conn: &mut sqlx::PgConnection, id: Uuid) -> Result<Self, QueryError> {
        sqlx::query_as::<_, Self>(
            r"UPDATE tasks
            SET status = $1,
                attempts = attempts + 1
            WHERE id = $2
            RETURNING *",
        )
        .bind(TaskStatus::Failed)
        .bind(id)
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not fail task from id")
    }

    pub async fn from_id(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as(r"SELECT * FROM tasks WHERE id = $1")
            .bind(id)
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not get task from id")
    }

    pub fn get_all<'a>() -> GetAllTasks<'a> {
        GetAllTasks::new()
    }

    pub fn pull_all_pending(
        max_attempts: i32,
        now: Option<DateTime<Utc>>,
    ) -> Paginated<PullAllPendingTasks> {
        Paginated::new(PullAllPendingTasks {
            max_attempts,
            now: now.unwrap_or_else(Utc::now),
        })
    }

    pub async fn requeue_stalled(
        conn: &mut sqlx::PgConnection,
        threshold: TimeDelta,
        now: Option<DateTime<Utc>>,
    ) -> Result<u64, QueryError> {
        sqlx::query(
            r"UPDATE tasks
            SET status = $1, updated_at = $2
            WHERE id IN (
                SELECT id
                FROM tasks
                WHERE status = $3 AND current_timestamp >=
                    TO_TIMESTAMP(EXTRACT(EPOCH FROM CASE WHEN last_retry IS NULL
                        THEN current_timestamp
                        ELSE last_retry
                    END) + EXTRACT(EPOCH FROM $4))
                FOR UPDATE SKIP LOCKED
            )",
        )
        .bind(TaskStatus::Queued)
        .bind(now)
        .bind(TaskStatus::Running)
        .bind(threshold)
        .execute(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not requeue stalled tasks")
        .map(|v| v.rows_affected())
    }
}

impl Task {
    pub async fn insert(
        conn: &mut sqlx::PgConnection,
        form: InsertTaskForm,
    ) -> Result<Self, QueryError> {
        // It has to be serialized before giving it to the database
        let data = serde_json::to_value(&form.data)
            .anonymize_error()
            .transform_context(QueryError)
            .attach_printable("could not serialize task to insert task")?;

        sqlx::query_as::<_, Task>(
            r"INSERT INTO tasks (id, deadline, attempts, periodic, priority, status, data)
            VALUES (COALESCE($1, gen_random_uuid()), $2, $3, $4, $5, $6, $7)
            RETURNING *",
        )
        .bind(form.id)
        .bind(form.deadline)
        .bind(form.attempts)
        .bind(form.periodic)
        .bind(form.priority)
        .bind(form.status)
        .bind(data)
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not insert task")
    }

    pub async fn update(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        form: UpdateTaskForm,
    ) -> Result<Option<Self>, QueryError> {
        // sqlx treated serde_json::Value value as jsonb type
        let data = match form.data {
            Some(n) => Some(
                serde_json::to_value(&n)
                    .anonymize_error()
                    .transform_context(QueryError)
                    .attach_printable("could not serialize task to insert task")?,
            ),
            None => None,
        };

        sqlx::query_as::<_, Task>(
            r"UPDATE tasks
            SET deadline = COALESCE($1, deadline),
                attempts = COALESCE($2, attempts),
                last_retry = COALESCE($3, last_retry),
                priority = COALESCE($4, priority),
                status = COALESCE($5, status),
                data = COALESCE($6, data),
                updated_at = $7
            WHERE id = $8
            RETURNING *",
        )
        .bind(form.deadline)
        .bind(form.attempts)
        .bind(form.last_retry)
        .bind(form.priority)
        .bind(form.status)
        .bind(data)
        // due to limitations with PullAllPendingTasks query, we have to
        // bind this argument to update `updated_at` manually.
        .bind(Utc::now())
        .bind(id)
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not update task from id")
    }

    pub async fn delete(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Task>(r"DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not delete task from id")
    }

    pub async fn delete_all(conn: &mut sqlx::PgConnection) -> Result<u64, QueryError> {
        sqlx::query(r"DELETE FROM tasks")
            .execute(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not delete all tasks")
            .map(|v| v.rows_affected())
    }

    pub async fn delete_all_with_status(
        conn: &mut sqlx::PgConnection,
        status: TaskStatus,
    ) -> Result<u64, QueryError> {
        sqlx::query(r"DELETE FROM tasks WHERE status = $1")
            .bind(status)
            .execute(conn)
            .await
            .change_context(QueryError)
            .attach_printable_lazy(|| format!("could not delete all tasks with status {status:?}"))
            .map(|v| v.rows_affected())
    }

    pub async fn delete_all_with_type(
        conn: &mut sqlx::PgConnection,
        task_type: &str,
    ) -> Result<u64, QueryError> {
        sqlx::query(r"DELETE FROM tasks WHERE data->>'type' = $1")
            .bind(task_type)
            .execute(conn)
            .await
            .change_context(QueryError)
            .attach_printable_lazy(|| format!("could not delete all tasks with type {task_type:?}"))
            .map(|v| v.rows_affected())
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use crate::schema::{TaskPriority, TaskRawData, TaskStatus};
    use crate::test_utils;

    use super::*;
    use chrono::Utc;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_requeue_stalled(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let now = Utc::now();
        let threshold = TimeDelta::seconds(5);

        let within_threshold = now + TimeDelta::seconds(3);
        let outside_threshold = now - TimeDelta::seconds(10);

        let task_1 = test_utils::generate_task(&mut conn).await?;
        let form = UpdateTaskForm::builder()
            .status(Some(TaskStatus::Running))
            .last_retry(Some(outside_threshold))
            .build();
        Task::update(&mut conn, task_1.id, form).await?;

        let task_2 = test_utils::generate_task(&mut conn).await?;
        let form = UpdateTaskForm::builder()
            .status(Some(TaskStatus::Running))
            .last_retry(Some(within_threshold))
            .build();
        Task::update(&mut conn, task_2.id, form).await?;

        let total = Task::requeue_stalled(&mut conn, threshold, Some(now)).await?;
        assert_eq!(total, 1);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let task = test_utils::generate_task(&mut conn).await?;

        assert!(Task::from_id(&mut conn, task.id)
            .await
            .anonymize_error()?
            .is_some());

        Task::delete(&mut conn, task.id).await.anonymize_error()?;

        assert!(Task::from_id(&mut conn, task.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let deadline = Utc::now();
        let data = TaskRawData {
            kind: "foo".into(),
            inner: serde_json::json!({
                "currency": "PHP",
                "deadline": Utc::now(),
                "payer_id": "613425648685547541",
                "price": 15.0,
            }),
        };

        let form = InsertTaskForm::builder()
            .deadline(deadline)
            .priority(TaskPriority::High)
            .data(data.clone())
            .build();

        // milisecond precision lost for this: assert_eq!(task.deadline, deadline);
        let task = Task::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(task.attempts, 0);
        assert!(!task.periodic);
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.status, TaskStatus::Queued);
        assert_eq!(task.data, data);

        // For periodic tasks
        let deadline = Utc::now();
        let data = TaskRawData {
            kind: "foo".into(),
            inner: serde_json::json!({
                "currency": "PHP",
                "deadline": Utc::now(),
                "payer_id": "613425648685547541",
                "price": 15.0,
            }),
        };

        let form = InsertTaskForm::builder()
            .deadline(deadline)
            .priority(TaskPriority::High)
            .periodic(true)
            .data(data.clone())
            .build();

        // milisecond precision lost for this: assert_eq!(task.deadline, deadline);
        let task = Task::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(task.attempts, 0);
        assert!(task.periodic);
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.status, TaskStatus::Queued);
        assert_eq!(task.data, data);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_update(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let task = test_utils::generate_task(&mut conn).await?;

        let new_deadline = Utc::now();
        let form = UpdateTaskForm::builder()
            .deadline(Some(new_deadline))
            .attempts(Some(2))
            .priority(Some(TaskPriority::Low))
            .status(Some(TaskStatus::Failed))
            .build();

        let new_data = Task::update(&mut conn, task.id, form)
            .await
            .anonymize_error()?;

        assert!(new_data.is_some());

        // milisecond precision lost for this: assert_eq!(new_data.deadline, new_deadline);
        let new_data = new_data.unwrap();
        assert!(new_data.updated_at.is_some());
        assert_eq!(new_data.attempts, 2);
        assert_eq!(new_data.priority, TaskPriority::Low);
        assert_eq!(new_data.status, TaskStatus::Failed);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_delete(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let task = test_utils::generate_task(&mut conn).await?;

        assert!(Task::from_id(&mut conn, task.id)
            .await
            .anonymize_error()?
            .is_some());

        Task::delete(&mut conn, task.id).await.anonymize_error()?;

        assert!(Task::from_id(&mut conn, task.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }
}
