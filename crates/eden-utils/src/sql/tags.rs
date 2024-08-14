use serde::{ser::SerializeMap, Deserialize, Serialize};

/// Represents all the ways that can fail to perform
/// database related operations.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DatabaseErrorType {
    /// This error caused by connection issues from or to the
    /// Postgres database while trying to open a new connection
    /// or encountered internal error from [SQLx](sqlx).
    PoolError,
    /// No rows returned from a query that is expected to return
    /// at least one row.
    RowNotFound,
    /// A query statement is timed out.
    StatementTimedOut,
    /// It is caused by a column that is expected to be unique, the
    /// value exists in within the database table or other table
    /// (if validated by a query trigger).
    UniqueViolation,
    /// The cause of error is unknown, maybe internal perhaps.
    Unknown,
}

impl DatabaseErrorType {
    pub(crate) fn install_hook() {
        crate::Error::install_serde_hook::<Self>();
        crate::Error::install_hook::<Self>(|_this, _ctx| {
            // practically nothing...
        });
    }
}

/// Contains PostgreSQL error data occurred when there's an error
/// from Postgres while performing database related operations.
#[derive(Debug)]
pub struct PostgresErrorInfo {
    pub(crate) code: Option<String>,
    pub(crate) message: String,
}

impl Serialize for PostgresErrorInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("_type", "POSTGRES_ERROR")?;
        map.serialize_entry("code", &self.code)?;
        map.serialize_entry("message", &self.message)?;
        map.end()
    }
}

impl PostgresErrorInfo {
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    #[must_use]
    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

impl PostgresErrorInfo {
    pub(crate) fn install_hook() {
        crate::Error::install_serde_hook::<Self>();
        crate::Error::install_hook::<Self>(|this, ctx| {
            if let Some(code) = this.code.as_deref() {
                ctx.push_body(format!("postgres error code: {code}"));
            }
        });
    }
}
