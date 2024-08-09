use chrono::{DateTime, TimeDelta, Utc};
use std::time::{Duration, Instant, SystemTime};

#[must_use]
pub fn later(delta: TimeDelta) -> DateTime<Utc> {
    Utc::now() + delta
}

pub trait IntoStdDuration {
    fn into_std_duration(self) -> Option<Duration>;
}

impl IntoStdDuration for TimeDelta {
    fn into_std_duration(self) -> Option<Duration> {
        self.to_std().or_else(|_| self.abs().to_std()).ok()
    }
}

pub trait InstantExt {
    /// Gets the timestamp of this [`Instant`] object was created.
    fn started(&self) -> DateTime<Utc>;
}

impl InstantExt for Instant {
    #[must_use]
    fn started(&self) -> DateTime<Utc> {
        let current_time = SystemTime::now();
        let starting_time = current_time
            .checked_sub(self.elapsed())
            .unwrap_or(current_time);

        DateTime::<Utc>::from(starting_time)
    }
}
