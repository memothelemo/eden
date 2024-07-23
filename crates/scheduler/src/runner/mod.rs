use chrono::{DateTime, TimeDelta, Utc};
use dashmap::DashMap;
use eden_utils::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

mod internal;
use self::internal::*;

use crate::Job;

#[derive(Clone)]
pub struct JobRunner<S> {
    registry: Arc<DashMap<&'static str, JobMetadata<S>>>,
    pool: sqlx::PgPool,
    state: S,
}

impl<S> JobRunner<S>
where
    S: Clone + Send + Sync + 'static,
{
    #[must_use]
    pub fn new(pool: sqlx::PgPool, state: S) -> Self {
        Self {
            registry: Arc::new(DashMap::new()),
            pool,
            state,
        }
    }

    pub fn push<J>(&self, _job: J) -> Result<()>
    where
        J: Job<State = S> + Serialize,
    {
        todo!()
    }

    pub fn schedule<J>(&self, _job: J, _schedule: Schedule) -> Result<()>
    where
        J: Job<State = S> + Serialize,
    {
        todo!()
    }

    pub fn register_job<J>(self) -> Self
    where
        J: Job<State = S> + DeserializeOwned,
    {
        if self.registry.contains_key(J::id()) {
            panic!("Job {:?} is already registered", J::id());
        }

        let deserializer: DeserializerFn<S> = Box::new(|value| {
            let job: J = serde_json::from_value(value)?;
            Ok(Box::new(job))
        });
        let metadata: JobMetadata<S> = JobMetadata {
            deserializer,
            schedule: Box::new(J::schedule),
        };

        self.registry.insert(J::id(), metadata);
        self
    }
}

impl<S> JobRunner<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn serialize_job_task_data() {}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Schedule {
    At(DateTime<Utc>),
    In(TimeDelta),
}

impl Schedule {
    #[must_use]
    pub const fn now() -> Self {
        Self::In(TimeDelta::zero())
    }

    #[must_use]
    pub fn timestamp(&self, now: Option<DateTime<Utc>>) -> DateTime<Utc> {
        match self {
            Schedule::At(n) => *n,
            Schedule::In(delta) => {
                let now = now.unwrap_or_else(Utc::now);
                now + *delta
            }
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

impl Schedule {
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
