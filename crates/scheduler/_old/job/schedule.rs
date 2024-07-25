use chrono::{DateTime, TimeDelta, Utc};
use eden_utils::{error::ResultExt, Result};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskSchedule {
    // If set to the [`Task::schedule`], this task is considered as
    // persistent so it should not be pushed into the database.
    None,
    Cron(cron_clock::Schedule),
    Interval(TimeDelta),
    Multiple(Vec<TaskSchedule>),
    #[cfg(test)]
    #[doc(hidden)]
    Timestamp(DateTime<Utc>),
}

#[derive(Debug, Error)]
#[error("invalid cron expression")]
pub struct InvalidCronExpr;

impl TaskSchedule {
    // #[must_use]
    // pub fn now() -> Self {
    //     Self::Timestamp(Utc::now())
    // }

    pub fn cron<T: AsRef<str>>(cron: T) -> Result<Self, InvalidCronExpr> {
        cron_clock::Schedule::from_str(cron.as_ref())
            .map(Self::Cron)
            .change_context(InvalidCronExpr)
    }

    #[must_use]
    pub fn interval(delta: TimeDelta) -> Self {
        Self::Interval(delta)
    }

    // #[must_use]
    // pub fn timestamp<Tz: TimeZone>(dt: DateTime<Tz>) -> Self {
    //     Self::Timestamp(dt.to_utc())
    // }

    // pub fn later(delta: TimeDelta, now: Option<DateTime<Utc>>) -> Option<Self> {
    //     let current_dt = now.unwrap_or_else(Utc::now);
    //     let later_dt = current_dt.checked_add_signed(delta)?;
    //     Some(Self::Timestamp(later_dt))
    // }

    #[must_use]
    pub fn is_periodic(&self) -> bool {
        match self {
            Self::Interval(..) | Self::Cron(..) => true,
            Self::Multiple(schedules) => schedules.iter().any(|v| v.is_periodic()),
            _ => false,
        }
    }

    pub fn upcoming(&self, now: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
        let now = now.unwrap_or_else(Utc::now);
        match self {
            Self::None => None,
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
    fn should_not_schedule_if_none() {
        let schedule = TaskSchedule::None;
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
