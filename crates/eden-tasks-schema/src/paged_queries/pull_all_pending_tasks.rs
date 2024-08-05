use chrono::{DateTime, Utc};
use eden_utils::error::exts::{IntoEdenResult, ResultExt};
use eden_utils::sql::{PageQueyer, Paginated, QueryError};
use eden_utils::Result;
use sqlx::postgres::PgArguments;
use sqlx::Arguments;

use crate::types::{Task, TaskStatus, WorkerId};

#[must_use]
pub struct PullAllPendingTasks {
    // how many tasks we can limit per query
    pub(crate) limit: u64,
    pub(crate) max_attempts: i32,
    pub(crate) now: DateTime<Utc>,
    pub(crate) worker_id: WorkerId,
}

impl PullAllPendingTasks {
    // 100 tasks is our default limit unfortunately :)
    pub const DEFAULT_LIMIT: u64 = 100;

    #[must_use]
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = limit;
        self
    }

    #[must_use]
    pub fn build(self) -> Paginated<Self> {
        Paginated::new(self)
    }
}

impl PageQueyer for PullAllPendingTasks {
    type Output = Task;

    fn build_args(&self) -> PgArguments {
        let mut args = PgArguments::default();
        args.add(TaskStatus::Running);
        args.add(self.max_attempts);
        args.add(self.now);
        args.add(TaskStatus::Queued);
        args.add(self.worker_id.total_sql());
        args.add(self.worker_id.assigned_sql());
        args
    }

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT * FROM tasks ")?;
        write!(f, "WHERE status = $1 AND attempts < $2 ")?;
        write!(f, "AND deadline <= $3 AND updated_at = $3 ")?;
        write!(f, "AND get_worker_id_from_task(task_number, $5) = $6")?;
        write!(
            f,
            "ORDER BY deadline, get_task_priority_level(priority) DESC "
        )?;
        write!(f, "FOR UPDATE SKIP LOCKED")
    }

    async fn prerun(&self, conn: &mut sqlx::PgConnection) -> Result<(), QueryError> {
        // this is to better differentiate which tasks are updated now
        sqlx::query(
            r"UPDATE tasks SET status = $1, updated_at = $3,
                last_retry = CASE WHEN attempts > 0
                    THEN $3
                    ELSE last_retry
                END
            WHERE id IN(
                SELECT id FROM tasks
                WHERE attempts < $2
                    AND deadline <= $3
                    AND status = $4
                    AND get_worker_id_from_task(task_number, $5) = $6
                LIMIT $7
            )",
        )
        .bind(TaskStatus::Running)
        .bind(self.max_attempts)
        .bind(self.now)
        .bind(TaskStatus::Queued)
        .bind(self.worker_id.total_sql())
        .bind(self.worker_id.assigned_sql())
        .bind((self.limit as i64).abs())
        .execute(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not pull queued tasks")?;

        Ok(())
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils;
    use crate::types::TaskPriority;
    use chrono::TimeDelta;
    use eden_utils::error::exts::AnonymizeErrorInto;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_pagination(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let later = Utc::now() + TimeDelta::seconds(200);
        test_utils::prepare_sample_tasks(&mut conn).await?;

        let mut stream = Task::pull_all_pending(WorkerId::ONE, 3, Some(later))
            .build()
            .size(3);

        let mut deadline_order_test = Vec::new();
        while let Some(tasks) = stream.next(&mut conn).await.anonymize_error()? {
            // deadlines are must be same each other in a page
            let deadline = tasks.first().unwrap().deadline;
            assert!(tasks.iter().all(|v| v.deadline == deadline));

            // it must be sorted from high to low
            assert_eq!(tasks.first().unwrap().priority, TaskPriority::High);
            assert_eq!(tasks.get(1).unwrap().priority, TaskPriority::Medium);
            assert_eq!(tasks.get(2).unwrap().priority, TaskPriority::Low);

            // they are all must be running
            assert!(tasks.iter().all(|v| v.status == TaskStatus::Running));
            deadline_order_test.push(deadline);
        }

        for n in deadline_order_test.windows(2) {
            assert!(n[0] < n[1]);
        }

        assert!(!deadline_order_test.is_empty());
        Ok(())
    }
}
