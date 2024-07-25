use chrono::{DateTime, Utc};
use eden_utils::error::ResultExt;
use eden_utils::Result;
use futures::Future;
use sqlx::postgres::PgArguments;
use sqlx::Arguments;

use crate::schema::{Task, TaskStatus};
use crate::utils::PagedQuery;
use crate::QueryError;

#[must_use]
pub struct PullAllPendingTasks {
    pub(crate) max_failed_attempts: i64,
    pub(crate) now: DateTime<Utc>,
}

impl PagedQuery for PullAllPendingTasks {
    type Output = Task;

    fn build_args(&self) -> PgArguments {
        let mut args = PgArguments::default();
        args.add(TaskStatus::Running);
        args.add(self.max_failed_attempts);
        args.add(self.now);
        args
    }

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT * FROM tasks ")?;
        write!(f, "WHERE status = $1 AND failed_attempts < $2 ")?;
        write!(f, "AND deadline <= $3 AND updated_at = $3 ")?;
        write!(f, "ORDER BY deadline, priority DESC ")?;
        write!(f, "FOR UPDATE SKIP LOCKED")
    }

    fn prerun(
        &self,
        conn: &mut sqlx::PgConnection,
    ) -> impl Future<Output = Result<(), QueryError>> + Send {
        // this is to better differentiate which tasks are updated now
        async {
            sqlx::query(
                r"UPDATE tasks SET status = $1, updated_at = $3,
                last_retry = CASE WHEN failed_attempts > 0
                    THEN $3
                    ELSE last_retry
                END
                WHERE failed_attempts < $2 AND deadline <= $3",
            )
            .bind(TaskStatus::Running)
            .bind(self.max_failed_attempts)
            .bind(self.now)
            .execute(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not pull queued tasks")?;

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::schema::TaskPriority;
    use crate::test_utils;
    use chrono::TimeDelta;
    use eden_utils::error::ResultExt;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_pagination(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let later = Utc::now() + TimeDelta::seconds(200);
        test_utils::prepare_sample_tasks(&mut conn).await?;

        let mut stream = Task::pull_all_pending(3, Some(later)).size(3);
        let mut deadline_order_test = Vec::new();

        while let Some(tasks) = stream.next(&mut conn).await.anonymize_error()? {
            // deadlines are must be same each other in a page
            let deadline = tasks.first().unwrap().deadline;
            assert!(tasks.iter().all(|v| v.deadline == deadline));

            // it must be sorted from high to low
            assert_eq!(tasks.get(0).unwrap().priority, TaskPriority::High);
            assert_eq!(tasks.get(1).unwrap().priority, TaskPriority::Medium);
            assert_eq!(tasks.get(2).unwrap().priority, TaskPriority::Low);

            // they are all must be running
            assert!(tasks.iter().all(|v| v.status == TaskStatus::Running));
            deadline_order_test.push(deadline);
        }

        for n in deadline_order_test.windows(2) {
            assert!(n[0] < n[1]);
        }

        assert!(deadline_order_test.len() > 0);
        Ok(())
    }
}
