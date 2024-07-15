mod id;

pub(crate) mod sql;
pub(crate) use self::id::*;

pub use self::sql::*;

use chrono::{DateTime, NaiveDateTime, Utc};

#[must_use]
pub(crate) fn naive_to_dt(dt: NaiveDateTime) -> DateTime<Utc> {
    DateTime::from_naive_utc_and_offset(dt, Utc)
}
