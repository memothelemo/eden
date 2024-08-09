use chrono::{DateTime, NaiveDateTime, Utc};
use eden_utils::sql::util::{naive_to_dt, SqlSnowflake};
use sqlx::Row;
use twilight_model::id::{marker::UserMarker, Id};

#[derive(Debug, Clone)]
pub struct User {
    pub id: Id<UserMarker>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub developer_mode: bool,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for User {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id = row.try_get::<SqlSnowflake<UserMarker>, _>("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;
        let developer_mode = row.try_get("developer_mode")?;

        Ok(Self {
            id: id.into(),
            created_at: naive_to_dt(created_at),
            updated_at: updated_at.map(naive_to_dt),
            developer_mode,
        })
    }
}
