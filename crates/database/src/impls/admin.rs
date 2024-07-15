use eden_utils::error::ResultExt;
use eden_utils::Result;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

use crate::forms::{InsertAdminForm, UpdateAdminForm};
use crate::schema::Admin;
use crate::utils::SqlSnowflake;
use crate::QueryError;

impl Admin {
    pub async fn from_id(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Admin>(r"SELECT * FROM admins WHERE id = $1 LIMIT 1")
            .bind(SqlSnowflake::new(id))
            .fetch_optional(conn)
            .await
            .change_context(QueryError)
            .attach_printable("could not get admin from id")
    }
}

impl Admin {
    pub async fn delete(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
    ) -> Result<Option<Self>, QueryError> {
        sqlx::query_as::<_, Admin>(
            r"DELETE FROM admins WHERE id = $1
            RETURNING *",
        )
        .bind(SqlSnowflake::new(id))
        .fetch_optional(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not delete admin from id")
    }

    pub async fn update(
        conn: &mut sqlx::PgConnection,
        id: Id<UserMarker>,
        form: UpdateAdminForm<'_>,
    ) -> Result<Self, QueryError> {
        sqlx::query_as::<_, Admin>(
            r"UPDATE admins
            SET name = $1
            WHERE id = $2
            RETURNING *",
        )
        .bind(form.name)
        .bind(SqlSnowflake::new(id))
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not update admin")
    }

    pub async fn insert(
        conn: &mut sqlx::PgConnection,
        form: InsertAdminForm<'_>,
    ) -> Result<Self, QueryError> {
        sqlx::query_as::<_, Admin>(
            r"INSERT INTO admins(id, name)
            VALUES ($1, $2)
            RETURNING *",
        )
        .bind(SqlSnowflake::new(form.id))
        .bind(form.name)
        .fetch_one(conn)
        .await
        .change_context(QueryError)
        .attach_printable("could not insert admin")
    }
}

#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_from_id(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let admin = crate::test_utils::generate_admin(&mut conn).await?;
        let found_admin = Admin::from_id(&mut conn, admin.id)
            .await
            .anonymize_error()?;

        assert!(found_admin.is_some());

        let found_admin = found_admin.unwrap();
        assert_eq!(admin.id, found_admin.id);
        assert_eq!(admin.name, found_admin.name);

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_delete(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let admin = crate::test_utils::generate_admin(&mut conn).await?;
        let found_admin = Admin::delete(&mut conn, admin.id).await.anonymize_error()?;
        assert!(found_admin.is_some());

        let found_admin = found_admin.unwrap();
        assert_eq!(admin.id, found_admin.id);
        assert_eq!(admin.name, found_admin.name);

        assert!(Admin::from_id(&mut conn, admin.id)
            .await
            .anonymize_error()?
            .is_none());

        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_update(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;
        let admin = crate::test_utils::generate_admin(&mut conn).await?;

        let form = UpdateAdminForm::builder().name("superman").build();
        let new_admin = Admin::update(&mut conn, admin.id, form)
            .await
            .anonymize_error()?;

        assert_eq!(new_admin.name, Some("superman".into()));
        Ok(())
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_insert(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error()?;

        let id = Id::new(442252698964721669);
        let name = "Clyde";

        let form = InsertAdminForm::builder().id(id).name(Some(name)).build();
        let admin = Admin::insert(&mut conn, form).await.anonymize_error()?;
        assert_eq!(admin.id, id);
        assert_eq!(admin.name, Some(name.into()));

        Ok(())
    }
}
