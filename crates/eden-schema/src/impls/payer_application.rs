use eden_utils::error::exts::*;
use eden_utils::sql::util::SqlSnowflake;
use eden_utils::sql::QueryError;
use eden_utils::Result;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;
use uuid::Uuid;

use crate::forms::{InsertPayerApplicationForm, UpdatePayerApplicationForm};
use crate::types::PayerApplication;

impl PayerApplication {
    pub async fn first_pending(conn: &mut sqlx::PgConnection) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(
            r"SELECT * FROM payer_applications WHERE accepted IS NULL ORDER BY created_at",
        )
        .fetch_optional(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not get first pending payer application")
    }

    // TODO: Solve the n-1 problem
    pub async fn before_pending(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        /*
        WITH entries
        AS (
            SELECT row_number() OVER (ORDER BY created_at), *
            FROM payer_applications
            WHERE accepted IS NULL
        )
        SELECT *
        FROM entries
        WHERE row_number IN (
            SELECT row_number - 1
            FROM entries
            WHERE id = $1
        );*/
        sqlx::query_as::<_, Self>(
            r"WITH entries AS (
            SELECT row_number() OVER (ORDER BY created_at), *
            FROM payer_applications
            WHERE accepted IS NULL
        )
        SELECT *
        FROM entries
        WHERE row_number IN (
            SELECT row_number - 1
            FROM entries
            WHERE id = $1
        )",
        )
        .bind(id)
        .fetch_optional(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not get previous pending payer application")
    }

    // TODO: Solve the n+1 problem
    pub async fn after_pending(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        /*
        WITH entries
        AS (
            SELECT row_number() OVER (ORDER BY created_at), *
            FROM payer_applications
            WHERE accepted IS NULL
        )
        SELECT *
        FROM entries
        WHERE row_number IN (
            SELECT row_number + 1
            FROM entries
            WHERE id = $1
        );*/
        sqlx::query_as::<_, Self>(
            r"WITH entries AS (
                SELECT row_number() OVER (ORDER BY created_at), *
                FROM payer_applications
                WHERE accepted IS NULL
            )
            SELECT *
            FROM entries
            WHERE row_number IN (
                SELECT row_number + 1
                FROM entries
                WHERE id = $1
            )",
        )
        .bind(id)
        .fetch_optional(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not get next pending payer application")
    }

    pub async fn from_id(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(r"SELECT * FROM payer_applications WHERE id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(conn)
            .await
            .into_eden_error()
            .change_context(QueryError)
            .attach_printable("could not get payer application from id")
    }

    pub async fn from_user_id(
        conn: &mut sqlx::PgConnection,
        user_id: Id<UserMarker>,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(r"SELECT * FROM payer_applications WHERE user_id = $1 LIMIT 1")
            .bind(SqlSnowflake::new(user_id))
            .fetch_optional(conn)
            .await
            .into_eden_error()
            .change_context(QueryError)
            .attach_printable("could not get payer application from user's id")
    }
}

impl PayerApplication {
    pub async fn delete(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(
            r"DELETE FROM payer_applications
            WHERE id = $1
            RETURNING *",
        )
        .bind(id)
        .fetch_optional(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not delete payer application")
    }

    pub async fn insert(
        conn: &mut sqlx::PgConnection,
        form: InsertPayerApplicationForm<'_>,
    ) -> Result<Self, QueryError> {
        sqlx::query_as::<_, Self>(
            r"INSERT INTO payer_applications(name, user_id, java_username, bedrock_username, answer, icon_url)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *",
        )
        .bind(form.name)
        .bind(SqlSnowflake::new(form.user_id))
        .bind(form.java_username)
        .bind(form.bedrock_username)
        .bind(form.answer)
        .bind(form.icon_url)
        .fetch_one(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not insert payer application")
    }

    pub async fn update(
        conn: &mut sqlx::PgConnection,
        id: Uuid,
        form: UpdatePayerApplicationForm<'_>,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(
            r"UPDATE payer_applications
            SET accepted = $1,
                deny_reason = $2
            WHERE id = $3
            RETURNING *",
        )
        .bind(form.accepted)
        .bind(form.deny_reason)
        .bind(id)
        .fetch_optional(conn)
        .await
        .into_eden_error()
        .change_context(QueryError)
        .attach_printable("could not update payer application")
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_delete(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let application = test_utils::generate_payer_application(&mut conn).await?;

        let result = PayerApplication::from_id(&mut conn, application.id).await?;
        assert!(result.is_some());

        PayerApplication::delete(&mut conn, application.id).await?;

        let result = PayerApplication::from_id(&mut conn, application.id).await?;
        assert!(result.is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_user_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let application = test_utils::generate_payer_application(&mut conn).await?;

        let result = PayerApplication::from_user_id(&mut conn, application.user_id).await?;
        assert!(result.is_some());

        PayerApplication::delete(&mut conn, application.id).await?;

        let result = PayerApplication::from_user_id(&mut conn, application.user_id).await?;
        assert!(result.is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let application = test_utils::generate_payer_application(&mut conn).await?;

        let result = PayerApplication::from_id(&mut conn, application.id).await?;
        assert!(result.is_some());

        PayerApplication::delete(&mut conn, application.id).await?;

        let result = PayerApplication::from_id(&mut conn, application.id).await?;
        assert!(result.is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_update(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let application = test_utils::generate_payer_application(&mut conn).await?;

        let form = UpdatePayerApplicationForm::builder()
            .accepted(true)
            .deny_reason("Bad boy. That's all. Thank you very much.")
            .build();

        let new = PayerApplication::update(&mut conn, application.id, form).await?;
        assert!(new.is_some());

        let new = new.unwrap();
        assert_eq!(new.accepted, Some(true));
        assert_eq!(
            new.deny_reason,
            Some("Bad boy. That's all. Thank you very much.".into())
        );
        assert!(new.updated_at.is_some());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert_with_bedrock_username(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;

        let user_id = Id::new(12345678);
        let name = "poopyy";
        let java_username = "fooooo";
        let bedrock_username = "fooooo_123";
        let answer = "I like strawberry pies";
        let icon_url = "https://example.com";

        let form = InsertPayerApplicationForm::builder()
            .user_id(user_id)
            .name(&name)
            .java_username(java_username)
            .bedrock_username(Some(bedrock_username))
            .answer(answer)
            .icon_url(icon_url)
            .build();

        let application = PayerApplication::insert(&mut conn, form).await?;

        assert_eq!(application.user_id, user_id);
        assert_eq!(application.name, name);
        assert_eq!(application.java_username, java_username);
        assert_eq!(
            application.bedrock_username,
            Some(bedrock_username.to_string())
        );
        assert_eq!(application.answer, answer);
        assert_eq!(application.icon_url, Some(icon_url.into()));

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;

        let user_id = Id::new(12345678);
        let name = "poopyy";
        let java_username = "fooooo";
        let answer = "I like strawberry pies";

        let form = InsertPayerApplicationForm::builder()
            .user_id(user_id)
            .name(&name)
            .java_username(java_username)
            .bedrock_username(None)
            .answer(answer)
            .icon_url("https://example.com")
            .build();

        let application = PayerApplication::insert(&mut conn, form).await?;

        assert_eq!(application.user_id, user_id);
        assert_eq!(application.name, name);
        assert_eq!(application.java_username, java_username);
        assert_eq!(application.bedrock_username, None);
        assert_eq!(application.answer, answer);
        assert_eq!(application.icon_url, Some("https://example.com".into()));

        Ok(())
    }
}
