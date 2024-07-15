use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use twilight_model::id::{marker::UserMarker, Id};

use crate::utils::{naive_to_dt, SqlSnowflake};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Bill {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub created_by: Id<UserMarker>,
    pub updated_at: Option<DateTime<Utc>>,

    pub currency: String,
    pub deadline: NaiveDate,
    pub price: Decimal,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Bill {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id = row.try_get("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let created_by = row.try_get::<SqlSnowflake<UserMarker>, _>("created_by")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;
        let currency = row.try_get("currency")?;
        let deadline = row.try_get("deadline")?;
        let price = row.try_get("price")?;

        Ok(Self {
            id,
            created_at: naive_to_dt(created_at),
            created_by: created_by.into(),
            updated_at: updated_at.map(naive_to_dt),
            currency,
            deadline,
            price,
        })
    }
}
