use eden_utils::sql::util::SqlSnowflake;
use eden_utils::sql::{PageQueyer, Paginated};
use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

use crate::types::Payment;

#[must_use]
pub struct GetAllPayments {
    pub(crate) bill_id: Option<i64>,
    pub(crate) payer_id: Option<Id<UserMarker>>,
}

impl GetAllPayments {
    #[allow(clippy::new_without_default)]
    pub(crate) fn new() -> Self {
        Self {
            bill_id: None,
            payer_id: None,
        }
    }

    pub fn bill_id(mut self, id: Option<i64>) -> Self {
        self.bill_id = id;
        self
    }

    pub fn payer_id(mut self, id: Option<Id<UserMarker>>) -> Self {
        self.payer_id = id;
        self
    }

    pub fn build(self) -> Paginated<Self> {
        Paginated::new(self)
    }
}

impl PageQueyer for GetAllPayments {
    type Output = Payment;

    fn build_args(&self) -> PgArguments {
        let mut args = PgArguments::default();
        if let Some(bill_id) = self.bill_id {
            args.add(bill_id);
        }
        if let Some(payer_id) = self.payer_id {
            args.add(SqlSnowflake::new(payer_id));
        }
        args
    }

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT * FROM payments")?;

        let mut count = 0;
        if self.bill_id.is_some() {
            count += 1;
            write!(f, " WHERE ")?;
            write!(f, "bill_id = ${count}")?;
        }

        if self.payer_id.is_some() {
            count += 1;
            if count > 0 {
                write!(f, " WHERE ")?;
            }
            write!(f, "payer_id = ${count}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;
    use eden_utils::{error::exts::AnonymizeErrorInto, sql::Paginated};

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_with_payers_filter(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let payer = test_utils::generate_payer(&mut conn).await?;

        for _ in 0..100 {
            let bill = test_utils::generate_bill(&mut conn).await?;
            test_utils::generate_payment(&mut conn, bill.id, payer.id).await?;
        }

        let target_bill = test_utils::generate_bill(&mut conn).await?;
        test_utils::generate_payment(&mut conn, target_bill.id, payer.id).await?;

        let mut stream = Paginated::new(GetAllPayments {
            bill_id: None,
            payer_id: Some(payer.id),
        })
        .size(10);

        let mut has_one = true;
        while let Some(data) = stream.next(&mut conn).await? {
            assert!(data.len() == 10 || data.len() == 1);
            has_one = has_one && data.iter().all(|v| v.payer_id == payer.id);
        }
        assert!(has_one);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_with_bills_filter(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let payer = test_utils::generate_payer(&mut conn).await?;

        for _ in 0..100 {
            let bill = test_utils::generate_bill(&mut conn).await?;
            test_utils::generate_payment(&mut conn, bill.id, payer.id).await?;
        }

        let target_bill = test_utils::generate_bill(&mut conn).await?;
        test_utils::generate_payment(&mut conn, target_bill.id, payer.id).await?;

        let mut stream = Paginated::new(GetAllPayments {
            bill_id: Some(target_bill.id),
            payer_id: None,
        })
        .size(10);

        let mut has_one = true;
        while let Some(data) = stream.next(&mut conn).await? {
            assert!(data.len() == 10 || data.len() == 1);
            has_one = has_one && data.iter().all(|v| v.bill_id == target_bill.id);
        }
        assert!(has_one);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_pagination(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let payer = test_utils::generate_payer(&mut conn).await?;

        for _ in 0..100 {
            let bill = test_utils::generate_bill(&mut conn).await?;
            test_utils::generate_payment(&mut conn, bill.id, payer.id).await?;
        }

        let mut stream = Paginated::new(GetAllPayments {
            bill_id: None,
            payer_id: None,
        })
        .size(10);

        while let Some(data) = stream.next(&mut conn).await? {
            assert_eq!(data.len(), 10);
        }

        Ok(())
    }
}
