use eden_utils::error::exts::*;
use eden_utils::sql::util::SqlSnowflake;
use eden_utils::sql::QueryError;
use eden_utils::Result;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

use crate::forms::{InsertIdentityForm, InsertPayerForm, UpdatePayerForm};
use crate::types::{Identity, Payer};

impl Payer {
    pub async fn from_id(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Payer>(r"SELECT * FROM payers WHERE id = $1 LIMIT 1")
            .bind(SqlSnowflake::new(id))
            .fetch_optional(conn)
            .await
            .into_eden_error()
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
            .attach_printable("could not delete all identities while trying to delete payer")?;

        sqlx::query_as::<_, Payer>(r"DELETE FROM payers WHERE id = $1")
            .bind(SqlSnowflake::new(id))
            .fetch_optional(conn)
            .await
            .into_eden_error()
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
        .fetch_optional(conn)
        .await
        .into_eden_error()
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
        .into_eden_error()
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
        .attach_printable("could not insert java identity for a payer")?;

        // Another one for bedrock
        // Default username from floodgate for bedrock users who haven't
        // linked their account yet, prefixed with a dot from a Java username
        let bedrock_username = form.bedrock_username.map_or_else(
            || format!(".{}", form.java_username),
            std::string::ToString::to_string,
        );

        // Hang on! Let's check if it is the same as Java because Postgres
        // will throw us a unique constraint violation error otherwise don't.
        if bedrock_username != form.java_username {
            Identity::insert(
                &mut *conn,
                InsertIdentityForm::builder()
                    .payer_id(form.id)
                    .name(Some(&bedrock_username))
                    .uuid(None)
                    .build(),
            )
            .await
            .attach_printable("could not insert bedrock identity for a payer")?;
        }

        Ok(payer)
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
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
        let mut conn = pool.acquire().await.anonymize_error_into()?;
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
        let mut conn = pool.acquire().await.anonymize_error_into()?;
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
    async fn test_insert_with_same_username_both_editions(
        pool: sqlx::PgPool,
    ) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;

        let id = Id::new(2345678);
        let name = "foo";

        let username = "foo123";
        let form = InsertPayerForm::builder()
            .id(id)
            .name(name)
            .java_username(username)
            .bedrock_username(Some(&username))
            .build();

        let payer = Payer::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(payer.id, id);
        assert_eq!(payer.name, name);

        let identities = Identity::get_all(payer.id)
            .next(&mut conn)
            .await
            .anonymize_error()?
            .unwrap();

        assert_eq!(identities.len(), 1);

        let identity = identities.first().unwrap();
        assert_eq!(identity.name, Some(username.into()));
        assert_eq!(identity.uuid, None);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert_with_bedrock_username(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;

        let id = Id::new(2345678);
        let name = "foo";

        let java_username = "foo123";
        let bedrock_username = "bar123";

        let form = InsertPayerForm::builder()
            .id(id)
            .name(name)
            .java_username(java_username)
            .bedrock_username(Some(bedrock_username))
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

        let bedrock_identity = identities.first().unwrap();
        assert_eq!(bedrock_identity.name, Some(bedrock_username.into()));
        assert_eq!(bedrock_identity.uuid, None);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;

        let id = Id::new(2345678);
        let name = "foo";

        let java_username = "foo123";
        let bedrock_username = ".foo123";

        let form = InsertPayerForm::builder()
            .id(id)
            .name(name)
            .java_username(java_username)
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

        let bedrock_identity = identities.first().unwrap();
        assert_eq!(bedrock_identity.name, Some(bedrock_username.into()));
        assert_eq!(bedrock_identity.uuid, None);

        Ok(())
    }
}
