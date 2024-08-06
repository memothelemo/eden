mod paginated;

pub mod error;
pub mod tags;
pub mod util;

pub use self::error::QueryError;
pub use self::error::{SqlErrorExt, SqlResultExt};
pub use self::paginated::*;

use self::tags::DatabaseErrorType;
use sqlx::error::ErrorKind;

use crate::error::any::report_from_any_error;
use crate::error::exts::{AnyErrorExt, IntoAnonymizedError, IntoError};
use crate::error::{Error, ErrorCategory};

#[derive(Debug)]
pub struct CountResult {
    pub total: i64,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for CountResult {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            total: row.try_get("total")?,
        })
    }
}

impl IntoError for sqlx::Error {
    type Context = QueryError;

    #[track_caller]
    fn into_eden_error(self) -> Error<QueryError> {
        self.into_eden_any_error().change_context(QueryError)
    }
}

impl IntoAnonymizedError for sqlx::Error {
    #[track_caller]
    fn into_eden_any_error(self) -> Error {
        let mut report = report_from_any_error(&self);
        let mut error_type = DatabaseErrorType::Unknown;

        match self {
            Self::Database(inner) => {
                match inner.kind() {
                    ErrorKind::UniqueViolation => {
                        // TODO: Parse unique violation data from error message
                        error_type = DatabaseErrorType::UniqueViolation;
                    }
                    _ => {}
                };

                let code = inner.code().map(|v| v.to_string());
                let message = inner.message().to_string();

                // special Postgres error codes can mean something also
                match code.as_deref() {
                    Some("57014") => error_type = DatabaseErrorType::StatementTimedOut,
                    _ => {}
                };

                // attaching PostgresErrorInfo
                report = report.attach(self::tags::PostgresErrorInfo { code, message });
            }
            Self::RowNotFound => {
                error_type = DatabaseErrorType::RowNotFound;
            }
            Self::PoolClosed | Self::PoolTimedOut => {
                error_type = DatabaseErrorType::PoolError;
            }
            _ => {}
        }

        Error::anonymized_report(ErrorCategory::Unknown, report.attach(error_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::error::exts::{AnonymizeErrorInto, IntoTypedError};
    use crate::Result;

    async fn generate_result(conn: &mut sqlx::PgConnection) -> Result<()> {
        sqlx::query("(")
            .execute(conn)
            .await
            .anonymize_error_into()?;

        Ok(())
    }

    #[sqlx::test]
    async fn should_classify_as_statement_timed_out(pool: sqlx::PgPool) -> Result<()> {
        let mut conn = pool.acquire().await.into_typed_error()?;
        sqlx::query(r#"set statement_timeout = '1000'"#)
            .execute(&mut *conn)
            .await
            .into_typed_error()?;

        let result = sqlx::query("select pg_sleep(3)")
            .execute(&mut *conn)
            .await
            .anonymize_error_into();

        assert!(result.is_err());
        assert_eq!(
            result.db_error_type(),
            Some(&DatabaseErrorType::StatementTimedOut)
        );

        Ok(())
    }

    #[sqlx::test]
    async fn should_classify_as_unique_violation(pool: sqlx::PgPool) -> Result<()> {
        let mut conn = pool.acquire().await.into_typed_error()?;

        sqlx::query(
            r#"create table "__sample__" (
                id serial primary key,
                name varchar unique not null
            )"#,
        )
        .execute(&mut *conn)
        .await
        .into_typed_error()?;

        sqlx::query(r#"insert into "__sample__" (name) values ('puppy');"#)
            .execute(&mut *conn)
            .await
            .into_typed_error()?;

        let result = sqlx::query(r#"insert into "__sample__" (name) values ('puppy')"#)
            .fetch_one(&mut *conn)
            .await
            .anonymize_error_into();

        assert!(result.is_err());
        assert_eq!(
            result.db_error_type(),
            Some(&DatabaseErrorType::UniqueViolation)
        );

        let pg_info = result.pg_error_info().unwrap();
        assert_eq!(pg_info.code, Some("23505".into()));
        assert_eq!(
            pg_info.message,
            r#"duplicate key value violates unique constraint "__sample___name_key""#
        );

        Ok(())
    }

    #[sqlx::test]
    async fn should_classify_as_row_not_found(pool: sqlx::PgPool) -> Result<()> {
        let mut conn = pool.acquire().await.into_typed_error()?;

        sqlx::query(
            r#"create table "__sample__" (
                id serial primary key,
                name varchar not null
            )"#,
        )
        .execute(&mut *conn)
        .await
        .into_typed_error()?;

        let result = sqlx::query(r#"select * from "__sample__""#)
            .fetch_one(&mut *conn)
            .await
            .anonymize_error_into();

        assert!(result.is_err());
        assert_eq!(
            result.db_error_type(),
            Some(&DatabaseErrorType::RowNotFound)
        );

        Ok(())
    }

    #[sqlx::test]
    async fn should_attached_database_error_type(pool: sqlx::PgPool) -> Result<()> {
        let mut conn = pool.acquire().await.anonymize_error_into()?;
        let result = generate_result(&mut conn).await;

        assert!(result.is_err());
        assert_eq!(result.db_error_type(), Some(&DatabaseErrorType::Unknown));

        Ok(())
    }
}
