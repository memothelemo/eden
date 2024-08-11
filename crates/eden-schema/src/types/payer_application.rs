use chrono::{DateTime, NaiveDateTime, Utc};
use eden_utils::sql::util::{naive_to_dt, SqlSnowflake};
use sqlx::Row;
use twilight_model::id::{marker::UserMarker, Id};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PayerApplication {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub name: String,
    pub user_id: Id<UserMarker>,
    pub java_username: String,
    pub bedrock_username: Option<String>,
    pub accepted: Option<bool>,
    pub answer: String,
    pub deny_reason: Option<String>,
    pub icon_url: Option<String>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for PayerApplication {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id = row.try_get("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;
        let name = row.try_get("name")?;
        let user_id = row.try_get::<SqlSnowflake<UserMarker>, _>("user_id")?;
        let java_username = row.try_get("java_username")?;
        let bedrock_username = row.try_get("bedrock_username")?;
        let accepted = row.try_get("accepted")?;
        let answer = row.try_get("answer")?;
        let deny_reason = row.try_get("deny_reason")?;
        let icon_url = row.try_get("icon_url")?;

        Ok(Self {
            id,
            created_at: naive_to_dt(created_at),
            updated_at: updated_at.map(naive_to_dt),
            name,
            user_id: user_id.into(),
            java_username,
            bedrock_username,
            accepted,
            answer,
            deny_reason,
            icon_url,
        })
    }
}
