use chrono::NaiveDate;
use once_cell::sync::Lazy;

pub struct BuildInfo {
    pub commit_branch: &'static str,
    pub commit_date: NaiveDate,
    pub commit_hash: &'static str,
    pub build_profile: &'static str,
}

#[allow(clippy::expect_used)]
pub static BUILD: Lazy<BuildInfo> = Lazy::new(|| {
    let commit_timestamp = env!("VERGEN_GIT_COMMIT_TIMESTAMP");
    let commit_date = chrono::DateTime::parse_from_rfc3339(commit_timestamp)
        .expect("commit timestamp contains invalid RFC 3339 timestamp")
        .naive_local()
        .date();

    BuildInfo {
        commit_branch: env!("VERGEN_GIT_BRANCH"),
        commit_date,
        commit_hash: env!("VERGEN_GIT_SHA"),
        build_profile: env!("BUILD_PROFILE"),
    }
});
