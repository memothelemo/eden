use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use twilight_model::id::{marker::UserMarker, Id};

use crate::utils::{naive_to_dt, SqlSnowflake};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Admin {
    pub id: Id<UserMarker>,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Admin {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let id = row.try_get::<SqlSnowflake<UserMarker>, _>("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let name = row.try_get("name")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;

        Ok(Self {
            id: id.into(),
            created_at: naive_to_dt(created_at),
            name,
            updated_at: updated_at.map(naive_to_dt),
        })
    }
}
