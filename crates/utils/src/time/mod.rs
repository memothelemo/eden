use chrono::{DateTime, Utc};
use std::time::{Instant, SystemTime};

use crate::internal::Sealed;

pub trait InstantExt: Sealed {
    /// Gets the timestamp of this [`Instant`] object was created.
    fn started(&self) -> DateTime<Utc>;
}

impl Sealed for Instant {}
impl InstantExt for Instant {
    #[allow(clippy::expect_used)]
    fn started(&self) -> DateTime<Utc> {
        let current_time = SystemTime::now();
        let starting_time = current_time
            .checked_sub(self.elapsed())
            .unwrap_or(current_time);

        DateTime::<Utc>::from(starting_time)
    }
}
