use chrono::{DateTime, NaiveDateTime, Utc};
use eden_utils::sql::util::{naive_to_dt, SqlSnowflake};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::ops::Deref;
use twilight_model::id::{marker::GuildMarker, Id};
use typed_builder::TypedBuilder;

#[derive(Debug)]
pub struct GuildSettingsRow {
    pub id: Id<GuildMarker>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub data: GuildSettings,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for GuildSettingsRow {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let id = row.try_get::<SqlSnowflake<GuildMarker>, _>("id")?;
        let created_at = row.try_get::<NaiveDateTime, _>("created_at")?;
        let updated_at = row.try_get::<Option<NaiveDateTime>, _>("updated_at")?;
        let data = row.try_get::<sqlx::types::Json<GuildSettings>, _>("data")?;

        Ok(Self {
            id: id.into(),
            created_at: naive_to_dt(created_at),
            updated_at: updated_at.map(naive_to_dt),
            data: data.0,
        })
    }
}

impl Deref for GuildSettingsRow {
    type Target = GuildSettings;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GuildSettingsVersion {
    #[default]
    V1,
}

// These fields may be changed in the future.
#[derive(Debug, Deserialize, Serialize, TypedBuilder, PartialEq, Eq)]
#[serde(default)]
pub struct GuildSettings {
    #[serde(rename = "_v")]
    #[builder(default)]
    pub version: GuildSettingsVersion,
    #[builder(default)]
    pub payers: PayerGuildSettings,
}

impl Default for GuildSettings {
    fn default() -> Self {
        Self {
            version: GuildSettingsVersion::V1,
            payers: PayerGuildSettings::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, TypedBuilder)]
#[serde(default)]
pub struct PayerGuildSettings {
    #[builder(default = false)]
    pub allow_self_register: bool,
}

impl Default for PayerGuildSettings {
    fn default() -> Self {
        Self {
            allow_self_register: true,
        }
    }
}
