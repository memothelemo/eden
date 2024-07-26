use chrono::{DateTime, TimeDelta, Utc};
use eden_utils::{error::ResultExt, Result};
use std::str::FromStr;

use crate::error::InvalidCronExpr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskSchedule {
    Once,
    Cron(cron_clock::Schedule),
    Interval(TimeDelta),
    Multiple(Vec<TaskSchedule>),
    #[cfg(test)]
    #[doc(hidden)]
    Timestamp(DateTime<Utc>),
}

impl TaskSchedule {
    pub fn cron<T: AsRef<str>>(cron: T) -> Result<Self, InvalidCronExpr> {
        cron_clock::Schedule::from_str(cron.as_ref())
            .map(Self::Cron)
            .change_context(InvalidCronExpr)
    }

    #[must_use]
    pub fn interval(delta: TimeDelta) -> Self {
        Self::Interval(delta)
    }

    #[must_use]
    pub fn is_periodic(&self) -> bool {
        match self {
            Self::Interval(..) | Self::Cron(..) => true,
            Self::Multiple(schedules) => schedules.iter().any(|v| v.is_periodic()),
            Self::Once => false,
            #[cfg(test)]
            Self::Timestamp(..) => false,
        }
    }

    pub fn upcoming(&self, now: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
        let now = now.unwrap_or_else(Utc::now);
        match self {
            Self::Once => None,
            Self::Cron(n) => n.after(&now).next(),
            Self::Interval(dt) => now.checked_add_signed(*dt),
            Self::Multiple(triggers) => triggers.iter().fold(None, |result, entry| {
                if let Some(trigger_next) = entry.upcoming(Some(now)) {
                    match result {
                        Some(current) if trigger_next < current => Some(trigger_next),
                        Some(current) => Some(current),
                        None => Some(trigger_next),
                    }
                } else {
                    result
                }
            }),
            #[cfg(test)]
            Self::Timestamp(n) => {
                let n = *n;
                if now <= n {
                    Some(n)
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_not_schedule_if_once() {
        let schedule = TaskSchedule::Once;
        assert_eq!(schedule.upcoming(None), None);
    }

    #[test]
    fn should_add_delta_if_interval() {
        let now = Utc::now();
        let delta = TimeDelta::seconds(5);
        let schedule = TaskSchedule::Interval(delta);

        let expected = now + delta;
        assert_eq!(schedule.upcoming(Some(now)), Some(expected));
    }

    #[test]
    fn should_use_timestamp_if_timestamp() {
        let now = Utc::now();
        let target = now + TimeDelta::days(3);
        let schedule = TaskSchedule::Timestamp(target);

        assert_eq!(schedule.upcoming(Some(now)), Some(target));

        let target = now - TimeDelta::days(3);
        let schedule = TaskSchedule::Timestamp(target);
        assert_eq!(schedule.upcoming(Some(now)), None);
    }

    #[test]
    fn multiple_schedules_should_work() {
        let now = Utc::now();
        let shortest = now + TimeDelta::days(2);
        let longest = now + TimeDelta::days(5);

        let schedule = TaskSchedule::Multiple(vec![
            TaskSchedule::Timestamp(shortest),
            TaskSchedule::Timestamp(longest),
        ]);
        assert_eq!(schedule.upcoming(Some(now)), Some(shortest));
    }
}
