use doku::Document;
use eden_utils::types::Sensitive;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sqlx::postgres::PgConnectOptions;
use std::str::FromStr;
use std::time::Duration as StdDuration;

#[serde_as]
#[derive(Debug, Document, Deserialize, Serialize)]
pub struct Database {
    /// Maximum amount of time to spend waiting for the database
    /// to successfully establish connection.
    ///
    /// Eden will reject any database related transactions if it exceeds
    /// the maximum amount of time waiting to successfully connect to
    /// the database.
    ///
    /// The default is `15` seconds, if not set.
    #[doku(as = "String", example = "15s")]
    #[serde(default = "Database::default_connect_timeout")]
    #[serde_as(as = "eden_utils::serial::AsHumanDuration")]
    pub(crate) connect_timeout: StdDuration,

    /// Maximum idle duration for individual pooled connections.
    ///
    /// Any connection remains idle longer than the configured
    /// will be closed.
    ///
    /// The default is `10` minutes, if not set.
    #[doku(as = "String", example = "10m")]
    #[serde(default = "Database::default_idle_timeout")]
    #[serde_as(as = "eden_utils::serial::AsHumanDuration")]
    pub(crate) idle_timeout: StdDuration,

    /// Maximum amount of connections for Eden to maintain it
    /// most of the time.
    ///
    /// The default is `10` connections, if not set.
    #[doku(example = "10")]
    #[serde(default = "Database::default_max_connections")]
    pub(crate) max_connections: u32,

    /// Minimum amount of connections for Eden to maintain it
    /// at all times.
    ///
    /// The minimum connections should not exceed to the maximum
    /// amount of comments (you may refer to max_connections, if you're
    /// unsure about its default value). However, the set value will be
    /// capped to `max_connections`.
    ///
    /// The default is `0` connections, if not set.
    #[doku(example = "0")]
    #[serde(default = "Database::default_min_connections")]
    pub(crate) min_connections: u32,

    /// Connection URL to connect to the Postgres database.
    ///
    /// You may want to refer to https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING
    /// for guide on how to setup connection URL or string to connect to the database.
    ///
    /// If your cloud provider provides connection URL/string to connect
    /// to the Postgres database, you should place this value here.
    #[doku(as = "String", example = "postgres://postgres@localhost/eden")]
    url: Sensitive<SerializableUrl>,
}

impl Database {
    #[must_use]
    pub fn connect_timeout(&self) -> StdDuration {
        self.connect_timeout
    }

    #[must_use]
    pub fn idle_timeout(&self) -> StdDuration {
        self.idle_timeout
    }

    #[must_use]
    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }

    #[must_use]
    pub fn min_connections(&self) -> u32 {
        self.min_connections
    }

    #[must_use]
    pub fn as_postgres_connect_options(&self) -> PgConnectOptions {
        self.url.as_ref().0.clone()
    }
}

impl Database {
    fn default_connect_timeout() -> StdDuration {
        StdDuration::from_secs(15)
    }

    fn default_idle_timeout() -> StdDuration {
        StdDuration::from_secs(60 * 10)
    }

    fn default_max_connections() -> u32 {
        10
    }

    fn default_min_connections() -> u32 {
        0
    }
}

struct SerializableUrl(PgConnectOptions);

impl<'de> serde::de::Deserialize<'de> for SerializableUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = SerializableUrl;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Postgres connection url")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                PgConnectOptions::from_str(v)
                    .map(SerializableUrl)
                    .map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl serde::Serialize for SerializableUrl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use sqlx::ConnectOptions;
        self.0.to_url_lossy().to_string().serialize(serializer)
    }
}
