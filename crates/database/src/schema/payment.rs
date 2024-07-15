use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value as Json;
use sqlx::Row;
use twilight_model::id::{marker::UserMarker, Id};
use uuid::Uuid;

use crate::payment::PaymentData;
use crate::utils::{naive_to_dt, SqlSnowflake};

#[derive(Debug, Clone)]
pub struct Payment {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub payer_id: Id<UserMarker>,
    pub bill_id: i64,
    pub data: PaymentData,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Payment {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id = row.try_get("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;

        let payer_id = row.try_get::<SqlSnowflake<UserMarker>, _>("payer_id")?;
        let bill_id = row.try_get("bill_id")?;

        let data = row.try_get::<Json, _>("data")?;
        let data = serde_json::from_value(data).map_err(|e| sqlx::Error::ColumnDecode {
            index: "data".into(),
            source: Box::new(e),
        })?;

        Ok(Self {
            id,
            created_at: naive_to_dt(created_at),
            updated_at: updated_at.map(naive_to_dt),
            payer_id: payer_id.into(),
            bill_id,
            data,
        })
    }
}
