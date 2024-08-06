use eden_utils::{error::exts::*, Result};

use crate::Bot;

// TODO: Add support for hybrid pool system with primary and backup databases
impl Bot {
    /// Obtain a database connection from the primary pool.
    #[tracing::instrument(skip_all)]
    pub async fn db_read(&self) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>> {
        self.pool
            .acquire()
            .await
            .anonymize_error_into()
            .attach_printable("could not obtain database connection")
    }

    /// Obtain a database transaction from the primary pool.
    #[tracing::instrument(skip_all)]
    pub async fn db_write(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
        self.pool
            .begin()
            .await
            .anonymize_error_into()
            .attach_printable("could not obtain database transaction")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eden_utils::sql::SqlErrorExt;
    use eden_utils::Result;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_statement_timeout() -> Result<()> {
        eden_utils::error::Error::init();

        let mut settings = crate::tests::generate_real_settings();
        settings.database.query_timeout = Duration::from_secs(2);

        let bot = Bot::new(Arc::new(settings));

        let mut conn = bot.db_read().await?;
        let result = sqlx::query("SELECT pg_sleep(3)")
            .execute(&mut *conn)
            .await
            .anonymize_error_into();

        assert!(result.is_statement_timed_out());
        Ok(())
    }
}
