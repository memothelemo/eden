use chrono::NaiveDate;
use std::sync::LazyLock;

pub const COMMIT_BRANCH: &str = env!("VERGEN_GIT_BRANCH");
pub const COMMIT_HASH: &str = env!("VERGEN_GIT_SHA");
pub const PROFILE: &str = env!("BUILD_PROFILE");
