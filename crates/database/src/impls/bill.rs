use eden_utils::error::ResultExt;
use eden_utils::Result;

use crate::forms::{InsertBillForm, UpdateBillForm};
use crate::paged_queries::GetAllBills;
use crate::schema::Bill;
use crate::utils::{Paginated, SqlSnowflake};
use crate::QueryError;

impl Bill {
    pub async fn from_id(
        conn: &mut sqlx::PgConnection,
        id: i64,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as(r"SELECT * FROM bills WHERE id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not get bill from id")
    }

    pub async fn from_latest(conn: &mut sqlx::PgConnection) -> Result<Option<Self>, QueryError> {
        sqlx::query_as(
            r"SELECT * FROM bills
            ORDER BY id DESC
            LIMIT 1",
        )
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not get latest bill")
    }

    pub fn get_all() -> Paginated<GetAllBills> {
        Paginated::new(GetAllBills)
    }
}

impl Bill {
    pub async fn update(
        conn: &mut sqlx::PgConnection,
        id: i64,
        form: UpdateBillForm<'_>,
    ) -> Result<Self, QueryError> {
        sqlx::query_as::<_, Bill>(
            r"UPDATE bills
            SET currency = COALESCE($1, currency),
                deadline = COALESCE($2, deadline),
                price = COALESCE($3, price)
            WHERE id = $4
            RETURNING *",
        )
        .bind(form.currency)
        .bind(form.deadline)
        .bind(form.price)
        .bind(id)
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not update bill")
    }

    pub async fn insert(
        conn: &mut sqlx::PgConnection,
        form: InsertBillForm<'_>,
    ) -> Result<Self, QueryError> {
        sqlx::query_as::<_, Bill>(
            r"INSERT INTO bills (created_by, currency, deadline, price)
            VALUES ($1, $2, $3, $4)
            RETURNING *",
        )
        .bind(SqlSnowflake::new(form.created_by))
        .bind(form.currency)
        .bind(form.deadline)
        .bind(form.price)
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not insert bill")
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::{prelude::FromPrimitive, Decimal};
    use twilight_model::id::Id;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let bill = crate::test_utils::generate_bill(&mut conn).await?;
        let found_bill = Bill::from_id(&mut conn, bill.id).await.anonymize_error()?;
        assert!(found_bill.is_some());

        let found_bill = found_bill.unwrap();
        assert_eq!(bill.id, found_bill.id);
        assert_eq!(bill.created_at, found_bill.created_at);
        assert_eq!(bill.created_by, found_bill.created_by);
        assert_eq!(bill.updated_at, found_bill.updated_at);
        assert_eq!(bill.currency, found_bill.currency);
        assert_eq!(bill.deadline, found_bill.deadline);
        assert_eq!(bill.price, found_bill.price);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_update(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let bill = crate::test_utils::generate_bill(&mut conn).await?;
        let form = UpdateBillForm::builder()
            .currency(Some("USD"))
            .price(Some(Decimal::from_f64(65.).unwrap()))
            .build();

        let new_bill = Bill::update(&mut conn, bill.id, form)
            .await
            .anonymize_error()?;

        assert_eq!(new_bill.created_at, bill.created_at);
        assert_eq!(new_bill.currency, "USD");
        assert_eq!(new_bill.price, Decimal::from_f64(65.).unwrap());
        assert_eq!(new_bill.deadline, bill.deadline);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let created_by = Id::new(123456);
        let currency = "PHP";
        let deadline = NaiveDate::from_ymd_opt(2023, 2, 10).unwrap();
        let price = Decimal::from_f64(20.).unwrap();

        let form = InsertBillForm::builder()
            .created_by(created_by)
            .currency(currency)
            .deadline(deadline)
            .price(price)
            .build();

        let bill = Bill::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(bill.created_by, created_by);
        assert_eq!(bill.currency, currency);
        assert_eq!(bill.deadline, deadline);
        assert_eq!(bill.price, price);

        Ok(())
    }
}
