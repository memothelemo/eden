use eden_utils::error::AnyResultExt;
use eden_utils::{error::ResultExt, Result};
use twilight_model::id::{marker::UserMarker, Id};
use uuid::Uuid;

use crate::forms::{InsertPaymentForm, UpdatePaymentForm};
use crate::paged_queries::GetAllPayments;
use crate::schema::Payment;
use crate::utils::SqlSnowflake;
use crate::QueryError;

impl Payment {
    pub fn get_all() -> GetAllPayments {
        GetAllPayments::new()
    }

    pub async fn get_from_payer_and_bill(
        conn: &mut sqlx::PgConnection,
        payer_id: Id<UserMarker>,
        bill_id: i64,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(
            r"SELECT * FROM payments
            WHERE payer_id = $1
            AND bill_id = $2",
        )
        .bind(SqlSnowflake::new(payer_id))
        .bind(bill_id)
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not get payment from bill and payer info")
    }

    pub async fn from_id(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(r"SELECT * FROM payments WHERE id = $1")
            .bind(id)
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not get payment from id")
    }
}

impl Payment {
    pub async fn insert(
        conn: &mut sqlx::PgConnection,
        form: InsertPaymentForm,
    ) -> Result<Self, QueryError> {
        // It has to be serialized before giving it to the database
        let data = serde_json::to_value(&form.data)
            .anonymize_error()
            .transform_context(QueryError)
            .attach_printable("could not serialize payment data to insert payment")?;

        sqlx::query_as::<_, Self>(
            r"INSERT INTO payments (payer_id, bill_id, data)
            VALUES ($1, $2, $3)
            RETURNING *",
        )
        .bind(SqlSnowflake::new(form.payer_id))
        .bind(form.bill_id)
        .bind(data)
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not update payment")
    }

    pub async fn update(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        form: UpdatePaymentForm,
    ) -> Result<Option<Self>, QueryError> {
        // It has to be serialized before giving it to the database
        let data = serde_json::to_value(&form.data)
            .anonymize_error()
            .transform_context(QueryError)
            .attach_printable("could not serialize payment data to update payment")?;

        sqlx::query_as::<_, Self>(
            r"
            UPDATE payments
            SET data = $1
            WHERE id = $2
            RETURNING *",
        )
        .bind(data)
        .bind(id)
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not update payment")
    }

    pub async fn delete(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(r"DELETE FROM payments WHERE id = $1")
            .bind(id)
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not delete payment")
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;

    use crate::payment::PaymentData;
    use crate::test_utils;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_get_from_payer_and_bill(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = test_utils::generate_payer(&mut conn).await?;
        let bill = test_utils::generate_bill(&mut conn).await?;

        assert!(
            Payment::get_from_payer_and_bill(&mut conn, payer.id, bill.id)
                .await
                .anonymize_error()?
                .is_none()
        );

        test_utils::generate_payment(&mut conn, bill.id, payer.id).await?;
        assert!(
            Payment::get_from_payer_and_bill(&mut conn, payer.id, bill.id)
                .await
                .anonymize_error()?
                .is_some()
        );

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = test_utils::generate_payer(&mut conn).await?;
        let bill = test_utils::generate_bill(&mut conn).await?;
        let payment = test_utils::generate_payment(&mut conn, bill.id, payer.id).await?;

        assert!(Payment::from_id(&mut conn, payment.id)
            .await
            .anonymize_error()?
            .is_some());

        Payment::delete(&mut conn, payment.id)
            .await
            .anonymize_error()?;

        assert!(Payment::from_id(&mut conn, payment.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = test_utils::generate_payer(&mut conn).await?;
        let bill = test_utils::generate_bill(&mut conn).await?;

        let data = PaymentData::builder()
            .method(test_utils::generate_paypal_payment())
            .build();

        let form = InsertPaymentForm::builder()
            .bill_id(bill.id)
            .payer_id(payer.id)
            .data(data.clone())
            .build();

        let payment = Payment::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(payment.bill_id, bill.id);
        assert_eq!(payment.payer_id, payer.id);
        assert_eq!(payment.data, data);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_update(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = test_utils::generate_payer(&mut conn).await?;
        let bill = test_utils::generate_bill(&mut conn).await?;
        let payment = test_utils::generate_payment(&mut conn, bill.id, payer.id).await?;

        let new_data = PaymentData::builder()
            .method(test_utils::generate_paypal_payment())
            .build();

        let change_form = UpdatePaymentForm::builder().data(new_data.clone()).build();
        let updated = Payment::update(&mut conn, payment.id, change_form)
            .await
            .anonymize_error()?;

        assert!(updated.is_some());
        assert_eq!(new_data.method, test_utils::generate_paypal_payment());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_delete(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = test_utils::generate_payer(&mut conn).await?;
        let bill = test_utils::generate_bill(&mut conn).await?;
        let payment = test_utils::generate_payment(&mut conn, bill.id, payer.id).await?;

        assert!(Payment::from_id(&mut conn, payment.id)
            .await
            .anonymize_error()?
            .is_some());

        Payment::delete(&mut conn, payment.id)
            .await
            .anonymize_error()?;

        assert!(Payment::from_id(&mut conn, payment.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }
}
