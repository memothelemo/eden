use crate::schema::Bill;
use crate::utils::PagedQuery;

pub struct GetAllBills;

impl PagedQuery for GetAllBills {
    type Output = Bill;

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT * FROM bills")
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
        for _ in 0..50 {
            test_utils::generate_bill(&mut conn).await?;
        }

        let mut stream = Paginated::new(GetAllBills).size(10);
        while let Some(data) = stream.next(&mut conn).await.anonymize_error()? {
            assert_eq!(data.len(), 10);
        }

        Ok(())
    }
}
