use thiserror::Error;

use super::tags::{DatabaseErrorType, PostgresErrorInfo};

#[derive(Debug, Error)]
#[error("could not perform query")]
pub struct QueryError;

pub trait SqlResultExt<E> {
    type Object: for<'a> sqlx::Decode<'a, sqlx::Postgres>;

    fn optional(self) -> crate::Result<Option<Self::Object>, E>;
}

impl<T, C> SqlResultExt<C> for crate::Result<T, C>
where
    T: for<'a> sqlx::Decode<'a, sqlx::Postgres>,
{
    type Object = T;

    fn optional(self) -> crate::Result<Option<Self::Object>, C> {
        match self {
            Ok(inner) => Ok(Some(inner)),
            Err(error) if error.is_row_not_found() => Ok(None),
            Err(error) => Err(error),
        }
    }
}

pub trait SqlErrorExt {
    fn db_error_type(&self) -> Option<&DatabaseErrorType>;
    fn pg_error_info(&self) -> Option<&PostgresErrorInfo>;

    fn is_pool_error(&self) -> bool {
        matches!(self.db_error_type(), Some(DatabaseErrorType::PoolError))
    }

    fn is_row_not_found(&self) -> bool {
        matches!(self.db_error_type(), Some(DatabaseErrorType::RowNotFound))
    }

    fn is_statement_timed_out(&self) -> bool {
        matches!(
            self.db_error_type(),
            Some(DatabaseErrorType::StatementTimedOut)
        )
    }

    fn is_unique_violation(&self) -> bool {
        matches!(
            self.db_error_type(),
            Some(DatabaseErrorType::UniqueViolation)
        )
    }
}

impl<T, C> SqlErrorExt for crate::Result<T, C> {
    fn db_error_type(&self) -> Option<&DatabaseErrorType> {
        match self {
            Ok(..) => None,
            Err(error) => error.db_error_type(),
        }
    }

    fn pg_error_info(&self) -> Option<&PostgresErrorInfo> {
        match self {
            Ok(..) => None,
            Err(error) => error.pg_error_info(),
        }
    }
}

impl<T> SqlErrorExt for crate::Error<T> {
    fn db_error_type(&self) -> Option<&DatabaseErrorType> {
        self.report.request_ref::<DatabaseErrorType>().next()
    }

    fn pg_error_info(&self) -> Option<&PostgresErrorInfo> {
        self.report.request_ref::<PostgresErrorInfo>().next()
    }
}
