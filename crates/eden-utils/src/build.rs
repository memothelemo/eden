use chrono::NaiveDate;
use std::sync::LazyLock;

pub const COMMIT_BRANCH: &str = env!("VERGEN_GIT_BRANCH");
pub const COMMIT_DATE: LazyLock<NaiveDate> = LazyLock::new(|| {
    let timestamp = env!("VERGEN_GIT_COMMIT_TIMESTAMP");
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .expect("commit timestamp contains invalid RFC 3339 timestamp")
        .naive_local()
        .date()
});
pub const COMMIT_HASH: &str = env!("VERGEN_GIT_SHA");
pub const PROFILE: &str = env!("BUILD_PROFILE");
