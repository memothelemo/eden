mod paginated;

pub mod error;
pub mod tags;

pub use self::error::{SqlErrorExt, SqlResultExt};
pub use self::paginated::*;

use self::error::QueryError;
use self::tags::DatabaseErrorType;
use sqlx::error::ErrorKind;

use crate::error::any::report_from_any_error;
use crate::error::exts::{AnyErrorExt, IntoAnonymizedError, IntoError};
use crate::error::{Error, ErrorCategory};

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
