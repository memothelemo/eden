use chrono::{DateTime, TimeDelta, Utc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scheduled {
    At(DateTime<Utc>),
    In(TimeDelta),
}

impl Scheduled {
    #[must_use]
    pub const fn now() -> Self {
        Self::In(TimeDelta::zero())
    }

    #[must_use]
    pub fn timestamp(&self, now: Option<DateTime<Utc>>) -> DateTime<Utc> {
        match self {
            Scheduled::At(n) => *n,
            Scheduled::In(delta) => {
                let now = now.unwrap_or_else(Utc::now);
                now + *delta
            }
        }
    }

    #[must_use]
    pub fn is_now(&self) -> bool {
        match self {
            Self::In(n) => n.is_zero(),
            _ => false,
        }
    }
}

macro_rules! delta_fns {
    ($name:ident, $s_name:literal, $l_name:literal) => {
        paste::paste! {
            /// Makes a new [`Scheduled`] with the given number of
            #[doc = $l_name]
            ///
            /// # Panics
            ///
            /// Refer to [`
            #[doc = $s_name]
            /// `] for panic conditions.
            #[inline]
            #[must_use]
            pub fn [<in_ $name>]($name: i64) -> Self {
                Self::In(TimeDelta::$name($name))
            }
        }
    };
    (optional: $name:ident, $s_name:literal, $l_name:literal) => {
        paste::paste! {
            /// Makes a new [`Scheduled`] with the given number of
            #[doc = $l_name]
            ///
            /// # Errors
            ///
            /// Refer to [`
            #[doc = $s_name]
            /// `] for error conditions.
            #[inline]
            #[must_use]
            pub fn [<try_in_ $name>]($name: i64) -> Option<Self> {
                TimeDelta::[<try_$name>]($name).map(Self::In)
            }
        }
    };
    { $( ($($tt:tt)*), )* } => {$( delta_fns!($($tt)*); )*};
}

impl Scheduled {
    delta_fns! {
        (weeks, "TimeDelta::weeks", "weeks"),
        (optional: weeks, "TimeDelta::try_weeks", "weeks"),

        (days, "TimeDelta::days", "days"),
        (optional: days, "TimeDelta::try_days", "days"),

        (hours, "TimeDelta::hours", "hours"),
        (optional: hours, "TimeDelta::try_hours", "hours"),

        (minutes, "TimeDelta::minutes", "minutes"),
        (optional: minutes, "TimeDelta::try_minutes", "minutes"),

        (seconds, "TimeDelta::seconds", "seconds"),
        (optional: seconds, "TimeDelta::try_seconds", "seconds"),
    }
}
