use eden_utils::sql::{util::SqlSnowflake, PageQueyer};
use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use twilight_model::id::{marker::UserMarker, Id};

use crate::types::IdentityView;

pub struct GetAllIdentities {
    pub(crate) payer_id: Id<UserMarker>,
}

impl PageQueyer for GetAllIdentities {
    type Output = IdentityView;

    fn build_args(&self) -> PgArguments {
        let mut args = PgArguments::default();
        args.add(SqlSnowflake::new(self.payer_id));
        args
    }

    fn build_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SELECT name, uuid FROM identities ")?;
        write!(f, "WHERE payer_id = $1")
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils;
    use eden_utils::{error::exts::AnonymizeErrorInto, sql::Paginated};

    use super::*;

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn test_pagination(pool: sqlx::PgPool) -> eden_utils::Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let payer = test_utils::generate_payer(&mut conn).await?;
        test_utils::generate_identity(&mut conn, payer.id).await?;

        let mut stream = Paginated::new(GetAllIdentities { payer_id: payer.id }).size(1);
        while let Some(data) = stream.next(&mut conn).await? {
            assert_eq!(data.len(), 1);
        }

        Ok(())
    }
}
