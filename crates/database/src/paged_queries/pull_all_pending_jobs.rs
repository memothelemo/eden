use chrono::{DateTime, Utc};
use eden_utils::error::ResultExt;
use eden_utils::Result;
use futures::Future;
use sqlx::postgres::PgArguments;
use sqlx::Arguments;

use crate::schema::{Job, JobStatus};
use crate::utils::PagedQuery;
use crate::QueryError;

#[must_use]
pub struct PullAllPendingJobs {
    pub(crate) max_failed_attempts: i64,
    pub(crate) now: DateTime<Utc>,
}

impl PagedQuery for PullAllPendingJobs {
    type Output = Job;

    fn build_args(&self) -> PgArguments {
        let mut args = PgArguments::default();
        args.add(JobStatus::Running);
        args.add(self.max_failed_attempts);
        args.add(self.now);
        args
    }

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT * FROM jobs ")?;
        write!(f, "WHERE status = $1 AND failed_attempts < $2 ")?;
        write!(f, "AND deadline <= $3 AND updated_at = $3 ")?;
        write!(f, "ORDER BY deadline, priority DESC ")?;
        write!(f, "FOR UPDATE SKIP LOCKED")
    }

    fn prerun(
        &self,
        conn: &mut sqlx::PgConnection,
    ) -> impl Future<Output = Result<(), QueryError>> + Send {
        // this is to better differentiate which jobs are updated now
        async {
            sqlx::query(
                r"UPDATE jobs SET status = $1, updated_at = $3
                WHERE failed_attempts < $2 AND deadline <= $3",
            )
            .bind(JobStatus::Running)
            .bind(self.max_failed_attempts)
            .bind(self.now)
            .execute(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not pull queued jobs")?;

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::schema::JobPriority;
    use crate::test_utils;
    use chrono::TimeDelta;
    use eden_utils::error::ResultExt;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_pagination(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let later = Utc::now() + TimeDelta::seconds(200);
        test_utils::prepare_sample_jobs(&mut conn).await?;

        let mut stream = Job::pull_all_pending(3, Some(later)).size(3);
        let mut deadline_order_test = Vec::new();

        while let Some(jobs) = stream.next(&mut conn).await.anonymize_error()? {
            // deadlines are must be same each other in a page
            let deadline = jobs.first().unwrap().deadline;
            assert!(jobs.iter().all(|v| v.deadline == deadline));

            // it must be sorted from high to low
            assert_eq!(jobs.get(0).unwrap().priority, JobPriority::High);
            assert_eq!(jobs.get(1).unwrap().priority, JobPriority::Medium);
            assert_eq!(jobs.get(2).unwrap().priority, JobPriority::Low);

            // they are all must be running
            assert!(jobs.iter().all(|v| v.status == JobStatus::Running));
            deadline_order_test.push(deadline);
        }

        for n in deadline_order_test.windows(2) {
            assert!(n[0] < n[1]);
        }

        assert!(deadline_order_test.len() > 0);
        Ok(())
    }
}
