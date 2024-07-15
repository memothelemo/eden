use eden_utils::error::ResultExt;
use eden_utils::Result;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

use crate::forms::InsertIdentityForm;
use crate::paged_queries::GetAllIdentities;
use crate::schema::Identity;
use crate::utils::{CountResult, Paginated, SqlSnowflake};
use crate::QueryError;

impl Identity {
    pub async fn from_id(
        conn: &mut sqlx::PgConnection,
        id: i64,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Self>(
            r"SELECT * FROM identities
            WHERE id = $1
            LIMIT 1",
        )
        .bind(id)
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not get identity from id")
    }

    pub fn get_all(payer_id: Id<UserMarker>) -> Paginated<GetAllIdentities> {
        Paginated::new(GetAllIdentities { payer_id })
    }

    pub async fn name_exists(
        conn: &mut sqlx::PgConnection,
        name: &str,
    ) -> Result<bool, QueryError> {
        sqlx::query_as::<_, Self>(
            r"SELECT * FROM identities
            WHERE name = $1
            LIMIT 1",
        )
        .bind(name)
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not check if name exists from identity")
        .map(|v| v.is_some())
    }

    pub async fn payer_total(
        conn: &mut sqlx::PgConnection,
        payer_id: Id<UserMarker>,
    ) -> Result<i64, QueryError> {
        sqlx::query_as::<_, CountResult>(
            r"SELECT count(*) AS total
            FROM identities WHERE payer_id = $1",
        )
        .bind(SqlSnowflake::new(payer_id))
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not get total of identities from payer")
        .map(|v| v.total)
    }
}

impl Identity {
    pub async fn insert(
        conn: &mut sqlx::PgConnection,
        form: InsertIdentityForm<'_>,
    ) -> Result<Self, QueryError> {
        sqlx::query_as::<_, Identity>(
            r"INSERT INTO identities (payer_id, name, uuid)
            VALUES ($1, $2, $3)
            RETURNING *",
        )
        .bind(SqlSnowflake::new(form.payer_id))
        .bind(form.name)
        .bind(form.uuid)
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not insert identity")
    }

    pub async fn delete(
        conn: &mut sqlx::PgConnection,
        id: i64,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Identity>(
            r"DELETE FROM identities WHERE id = $1
            RETURNING *",
        )
        .bind(id)
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not delete identity from id")
    }

    pub async fn delete_all(
        conn: &mut sqlx::PgConnection,
        payer_id: Id<UserMarker>,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Identity>(
            r"DELETE FROM identities WHERE payer_id = $1
            RETURNING *",
        )
        .bind(SqlSnowflake::new(payer_id))
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not delete identity from payer id")
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = crate::test_utils::generate_payer(&mut conn).await?;

        let id1 = crate::test_utils::generate_identity(&mut conn, payer.id).await?;
        let id2 = crate::test_utils::generate_identity_with_name(
            &mut conn,
            payer.id,
            "microbar_sandwich",
        )
        .await?;

        assert!(Identity::from_id(&mut conn, id1.id)
            .await
            .anonymize_error()?
            .is_some());

        assert!(Identity::from_id(&mut conn, id2.id)
            .await
            .anonymize_error()?
            .is_some());

        Identity::delete(&mut conn, id2.id)
            .await
            .anonymize_error()?;

        assert!(Identity::from_id(&mut conn, id2.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_name_exists(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = crate::test_utils::generate_payer(&mut conn).await?;

        let id = crate::test_utils::generate_identity(&mut conn, payer.id).await?;
        assert!(Identity::name_exists(&mut conn, &id.name.unwrap())
            .await
            .anonymize_error()?);

        assert!(!Identity::name_exists(&mut conn, ".")
            .await
            .anonymize_error()?);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_payer_total(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let payer = crate::test_utils::generate_payer(&mut conn).await?;
        let id1 = crate::test_utils::generate_identity_with_name(
            &mut conn,
            payer.id,
            "test_identityyy_1",
        )
        .await?;

        assert_eq!(
            Identity::payer_total(&mut conn, payer.id)
                .await
                .anonymize_error()?,
            3
        );

        crate::test_utils::generate_identity_with_name(&mut conn, payer.id, "test_identityyy_2")
            .await?;

        assert_eq!(
            Identity::payer_total(&mut conn, payer.id)
                .await
                .anonymize_error()?,
            4
        );

        Identity::delete(&mut conn, id1.id)
            .await
            .anonymize_error()?;

        assert_eq!(
            Identity::payer_total(&mut conn, payer.id)
                .await
                .anonymize_error()?,
            3
        );

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_delete_all(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = crate::test_utils::generate_payer(&mut conn).await?;
        crate::test_utils::generate_identity(&mut conn, payer.id).await?;

        assert_eq!(
            Identity::payer_total(&mut conn, payer.id)
                .await
                .anonymize_error()?,
            3
        );

        Identity::delete_all(&mut conn, payer.id)
            .await
            .anonymize_error()?;

        assert_eq!(
            Identity::payer_total(&mut conn, payer.id)
                .await
                .anonymize_error()?,
            0
        );

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_delete(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = crate::test_utils::generate_payer(&mut conn).await?;
        let other_identity = crate::test_utils::generate_identity(&mut conn, payer.id).await?;

        Identity::delete(&mut conn, other_identity.id)
            .await
            .anonymize_error()?;

        assert!(Identity::from_id(&mut conn, other_identity.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let payer = crate::test_utils::generate_payer(&mut conn).await?;
        let name = "Clyde";
        let uuid = Uuid::new_v4();

        let form = InsertIdentityForm::builder()
            .name(Some(name))
            .payer_id(payer.id)
            .uuid(Some(uuid))
            .build();

        let identity = Identity::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(identity.name, Some(name.into()));
        assert_eq!(identity.payer_id, payer.id);
        assert_eq!(identity.uuid, Some(uuid));

        Ok(())
    }
}
