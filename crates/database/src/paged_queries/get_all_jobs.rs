use crate::schema::Job;
use crate::utils::PagedQuery;

#[must_use]
pub struct GetAllJobs;

impl PagedQuery for GetAllJobs {
    type Output = Job;

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SELECT * FROM jobs ")?;
        f.write_str("FOR UPDATE SKIP LOCKED")
    }
}

#[cfg(test)]
mod tests {
    use crate::{test_utils, utils::Paginated};
    use eden_utils::error::ResultExt;

    use super::*;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_pagination(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        test_utils::prepare_sample_jobs(&mut conn).await?;

        let mut stream = Paginated::new(GetAllJobs).size(3);
        while let Some(data) = stream.next(&mut conn).await.anonymize_error()? {
            assert_eq!(data.len(), 3);
        }

        Ok(())
    }
}
