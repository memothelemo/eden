use eden_utils::error::exts::*;
use eden_utils::sql::util::SqlSnowflake;
use eden_utils::sql::QueryError;
use eden_utils::Result;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

use crate::types::{GuildSettings, GuildSettingsRow};

impl GuildSettings {
    pub async fn from_guild(
        conn: &mut sqlx::PgConnection,
        id: Id<GuildMarker>,
    ) -> Result<GuildSettingsRow, QueryError> {
        // It has to be serialized before giving it to the database
        let data = serde_json::to_value(&GuildSettings::default())
            .into_typed_error()
            .change_context(QueryError)
            .attach_printable("could not serialize settings to insert guild settings")?;

        sqlx::query_as::<_, GuildSettingsRow>(
            r"INSERT INTO guild_settings(id, data)
            VALUES ($1, $2)
            ON CONFLICT (id) DO UPDATE
                SET updated_at = (now() at TIME ZONE ('utc'))
            RETURNING *",
        )
        .bind(SqlSnowflake::new(id))
        .bind(data)
        .fetch_one(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not get guild settings from guild id")
    }

    pub async fn update(
        conn: &mut sqlx::PgConnection,
        id: Id<GuildMarker>,
        data: &GuildSettings,
    ) -> Result<Option<GuildSettingsRow>, QueryError> {
        // It has to be serialized before giving it to the database
        let data = serde_json::to_value(data)
            .into_typed_error()
            .change_context(QueryError)
            .attach_printable("could not serialize settings to update guild settings")?;

        sqlx::query_as::<_, GuildSettingsRow>(
            r"UPDATE guild_settings
            SET data = $1
            WHERE id = $2
            RETURNING *",
        )
        .bind(data)
        .bind(SqlSnowflake::new(id))
        .fetch_optional(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not update guild settings")
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PayerGuildSettings;

    async fn is_exists(conn: &mut sqlx::PgConnection, id: Id<GuildMarker>) -> Result<bool> {
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT * FROM guild_settings WHERE id = $1)")
            .bind(SqlSnowflake::new(id))
            .fetch_one(conn)
            .await
            .anonymize_error_into()
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_update(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let guild_id = Id::<GuildMarker>::new(12345678);

        // Should insert if it doesn't exists
        GuildSettings::from_guild(&mut conn, guild_id)
            .await
            .anonymize_error()?;

        let data = GuildSettings::builder()
            .payers(
                PayerGuildSettings::builder()
                    .allow_self_register(false)
                    .build(),
            )
            .build();

        let new = GuildSettings::update(&mut conn, guild_id, &data)
            .await
            .anonymize_error()?;

        assert!(new.is_some());
        assert_eq!(new.unwrap().data, data);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_guild(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let guild_id = Id::<GuildMarker>::new(12345678);
        assert!(!is_exists(&mut conn, guild_id).await?);

        // Should insert if it doesn't exists
        GuildSettings::from_guild(&mut conn, guild_id)
            .await
            .anonymize_error()?;

        assert!(is_exists(&mut conn, guild_id).await?);

        // Should get the row if it does exists
        GuildSettings::from_guild(&mut conn, guild_id)
            .await
            .anonymize_error()?;

        Ok(())
    }
}
