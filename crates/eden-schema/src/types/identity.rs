use chrono::{DateTime, NaiveDateTime, Utc};
use eden_utils::sql::util::{naive_to_dt, SqlSnowflake};
use twilight_model::id::{marker::UserMarker, Id};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Identity {
    pub id: i64,
    pub payer_id: Id<UserMarker>,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
    pub uuid: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct IdentityView {
    pub name: Option<String>,
    pub uuid: Option<Uuid>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Identity {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let id = row.try_get::<i64, _>("id")?;
        let payer_id = row.try_get::<SqlSnowflake<UserMarker>, _>("payer_id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;

        let name = row.try_get::<Option<String>, _>("name")?;
        let uuid = row.try_get::<Option<Uuid>, _>("uuid")?;

        Ok(Self {
            id,
            payer_id: payer_id.into(),
            created_at: naive_to_dt(created_at),
            name,
            uuid,
        })
    }
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for IdentityView {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let name = row.try_get::<Option<String>, _>("name")?;
        let uuid = row.try_get::<Option<Uuid>, _>("uuid")?;

        Ok(Self { name, uuid })
    }
}
