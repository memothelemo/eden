use eden_utils::error::ResultExt;
use eden_utils::Result;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

use crate::forms::{InsertIdentityForm, InsertPayerForm, UpdatePayerForm};
use crate::schema::{Identity, Payer};
use crate::utils::SqlSnowflake;
use crate::QueryError;

impl Payer {
    pub async fn from_id(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Payer>(r"SELECT * FROM payers WHERE id = $1 LIMIT 1")
            .bind(SqlSnowflake::new(id))
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not get payer from id")
    }
}

impl Payer {
    pub async fn delete(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
    ) -> Result<Option<Self>, QueryError> {
        // We need to delete their identities first
        Identity::delete_all(&mut *conn, id)
            .await
            .change_context(QueryError)
            .attach_printable("could not delete all identities while trying to delete payer")?;

        sqlx::query_as::<_, Payer>(r"DELETE FROM payers WHERE id = $1")
            .bind(SqlSnowflake::new(id))
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not delete payer")
    }

    pub async fn update(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
        form: UpdatePayerForm<'_>,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Payer>(
            r"UPDATE payers
            SET name = $1
            WHERE id = $2
            RETURNING *",
        )
        .bind(form.name)
        .bind(SqlSnowflake::new(id))
        .fetch_optional(&mut *conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not update payer")
    }

    pub async fn insert(
        conn: &mut sqlx::PgConnection,
        form: InsertPayerForm<'_>,
    ) -> Result<Self, QueryError> {
        let payer = sqlx::query_as::<_, Payer>(
            r"INSERT INTO payers(id, name)
            VALUES ($1, $2)
            RETURNING *",
        )
        .bind(SqlSnowflake::new(form.id))
        .bind(form.name)
        .fetch_one(&mut *conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not insert payer")?;

        // Inserting java identity
        Identity::insert(
            &mut *conn,
            InsertIdentityForm::builder()
                .payer_id(form.id)
                .name(Some(form.java_username))
                .uuid(None)
                .build(),
        )
        .await
        .change_context(QueryError)
        .attach_printable("could not insert java identity for a payer")?;

        // Another one for bedrock
        // Default username from floodgate for bedrock users who haven't
        // linked their account yet, prefixed with a dot from a Java username
        let bedrock_username = form.bedrock_username.map_or_else(
            || format!(".{}", form.java_username),
            std::string::ToString::to_string,
        );

        Identity::insert(
            &mut *conn,
            InsertIdentityForm::builder()
                .payer_id(form.id)
                .name(Some(&bedrock_username))
                .uuid(None)
                .build(),
        )
        .await
        .change_context(QueryError)
        .attach_printable("could not insert bedrock identity for a payer")?;

        Ok(payer)
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = crate::test_utils::generate_payer(&mut conn).await?;

        assert!(Payer::from_id(&mut conn, payer.id)
            .await
            .anonymize_error()?
            .is_some());

        Payer::delete(&mut conn, payer.id).await.anonymize_error()?;
        assert!(Payer::from_id(&mut conn, payer.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_delete(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = crate::test_utils::generate_payer(&mut conn).await?;

        assert!(Payer::from_id(&mut conn, payer.id)
            .await
            .anonymize_error()?
            .is_some());

        Payer::delete(&mut conn, payer.id).await.anonymize_error()?;
        assert!(Payer::from_id(&mut conn, payer.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_update(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = crate::test_utils::generate_payer(&mut conn).await?;

        let new_name = "bar124".to_string();
        let form = UpdatePayerForm::builder().name(&new_name).build();

        let new_info = Payer::update(&mut conn, payer.id, form)
            .await
            .anonymize_error()?;

        assert!(new_info.is_some());

        let new_info = new_info.unwrap();
        assert_eq!(new_info.name, new_name);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert_with_bedrock_username(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let id = Id::new(2345678);
        let name = "foo";

        let java_username = "foo123";
        let bedrock_username = "bar123";

        let form = InsertPayerForm::builder()
            .id(id)
            .name(&name)
            .java_username(&java_username)
            .bedrock_username(Some(&bedrock_username))
            .build();

        let payer = Payer::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(payer.id, id);
        assert_eq!(payer.name, name);

        let identities = Identity::get_all(payer.id)
            .next(&mut conn)
            .await
            .anonymize_error()?
            .unwrap();

        let java_identity = identities.get(1).unwrap();
        assert_eq!(java_identity.name, Some(java_username.into()));
        assert_eq!(java_identity.uuid, None);

        let bedrock_identity = identities.get(0).unwrap();
        assert_eq!(bedrock_identity.name, Some(bedrock_username.into()));
        assert_eq!(bedrock_identity.uuid, None);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let id = Id::new(2345678);
        let name = "foo";

        let java_username = "foo123";
        let bedrock_username = ".foo123";

        let form = InsertPayerForm::builder()
            .id(id)
            .name(&name)
            .java_username(&java_username)
            .build();

        let payer = Payer::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(payer.id, id);
        assert_eq!(payer.name, name);

        let identities = Identity::get_all(payer.id)
            .next(&mut conn)
            .await
            .anonymize_error()?
            .unwrap();

        let java_identity = identities.get(1).unwrap();
        assert_eq!(java_identity.name, Some(java_username.into()));
        assert_eq!(java_identity.uuid, None);

        let bedrock_identity = identities.get(0).unwrap();
        assert_eq!(bedrock_identity.name, Some(bedrock_username.into()));
        assert_eq!(bedrock_identity.uuid, None);

        Ok(())
    }
}
