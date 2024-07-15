use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::Row;
use twilight_model::id::{marker::UserMarker, Id};

use crate::utils::{naive_to_dt, SqlSnowflake};

#[derive(Debug, Clone)]
pub struct Payer {
    pub id: Id<UserMarker>,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub updated_at: Option<DateTime<Utc>>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Payer {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id = row.try_get::<SqlSnowflake<UserMarker>, _>("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;
        let name = row.try_get("name")?;

        Ok(Self {
            id: id.into(),
            created_at: naive_to_dt(created_at),
            name,
            updated_at: updated_at.map(naive_to_dt),
        })
    }
}
