use eden_utils::error::exts::*;
use eden_utils::sql::util::SqlSnowflake;
use eden_utils::sql::QueryError;
use eden_utils::Result;
use tracing::trace;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

use crate::forms::UpdateUserForm;
use crate::types::User;

impl User {
    pub async fn get_or_insert(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
    ) -> Result<Self, QueryError> {
        let output = sqlx::query_as::<_, Self>(r#"SELECT * FROM "user" WHERE id = $1 LIMIT 1"#)
            .bind(SqlSnowflake::new(id))
            .fetch_optional(&mut *conn)
            .await
            .into_eden_error()
            .change_context(QueryError)
            .attach_printable("could not get user from id")?;

        if let Some(output) = output {
            Ok(output)
        } else {
            trace!("user {id} does not exist, creating one...");
            User::insert(conn, id).await
        }
    }
}

impl User {
    pub async fn delete(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(r#"DELETE FROM "user" WHERE id = $1"#)
            .bind(SqlSnowflake::new(id))
            .fetch_optional(conn)
            .await
            .into_eden_error()
            .change_context(QueryError)
            .attach_printable("could not delete user")
    }

    pub async fn update(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
        form: UpdateUserForm,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE "user"
            SET developer_mode = COALESCE($2, developer_mode)
            WHERE id = $1
            RETURNING *"#,
        )
        .bind(SqlSnowflake::new(id))
        .bind(form.developer_mode)
        .fetch_optional(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not update payer")
    }

    pub async fn insert(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
    ) -> Result<Self, QueryError> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO "user"(id)
            VALUES ($1)
            RETURNING *"#,
        )
        .bind(SqlSnowflake::new(id))
        .fetch_one(&mut *conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not insert user")
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(_pool: sqlx::PgPool) -> eden_utils::Result<()> {
        // TODO: Test this case here
        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_update(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let payer = crate::test_utils::generate_user(&mut conn).await?;

        let form = UpdateUserForm::builder().developer_mode(Some(true)).build();
        let new_info = User::update(&mut conn, payer.id, form)
            .await
            .anonymize_error()?;

        assert!(new_info.is_some());

        let new_info = new_info.unwrap();
        assert_eq!(new_info.developer_mode, true);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;

        let id = Id::new(2345678);
        let user = User::insert(&mut conn, id).await.anonymize_error()?;
        assert_eq!(user.id, id);

        Ok(())
    }
}
