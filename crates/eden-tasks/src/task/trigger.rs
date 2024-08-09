use chrono::{DateTime, TimeDelta, Utc};
use eden_utils::error::exts::{IntoTypedError, ResultExt};
use eden_utils::Result;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskTrigger {
    /// This trigger can be used to queue tasks with this trigger
    /// in demand depending on the schedule given.
    ///
    /// In other trigger types, this variant is only allowed to do
    /// that unless a recurring (set other than [`TaskTrigger::None`])
    /// task failed.
    None,

    /// This trigger is based on the cron expression and will be
    /// triggered if it reaches the cron expression's upcoming time.
    Cron(cron_clock::Schedule),

    /// This will be triggered in every <duration> periodically.
    Interval(TimeDelta),

    /// This will be triggered if any multiple triggers are triggered.
    Multiple(Vec<TaskTrigger>),
}

#[derive(Debug, Error)]
#[error("invalid cron expression")]
pub struct InvalidCronExpr;

impl TaskTrigger {
    pub fn cron<T: AsRef<str>>(cron: T) -> Result<Self, InvalidCronExpr> {
        cron_clock::Schedule::from_str(cron.as_ref())
            .map(Self::Cron)
            .into_typed_error()
            .change_context(InvalidCronExpr)
    }

    #[must_use]
    pub fn interval(delta: TimeDelta) -> Self {
        Self::Interval(delta)
    }

    #[must_use]
    pub fn is_recurring(&self) -> bool {
        match self {
            Self::Interval(..) | Self::Cron(..) => true,
            Self::Multiple(schedules) => schedules.iter().any(TaskTrigger::is_recurring),
            Self::None => false,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_not_get_upcoming_timestamp_if_none() {
        let schedule = TaskTrigger::None;
        assert_eq!(schedule.upcoming(None), None);
    }

    #[test]
    fn should_add_duration_from_now_if_interval() {
        let now = Utc::now();
        let delta = TimeDelta::seconds(5);
        let schedule = TaskTrigger::Interval(delta);

        let expected = now + delta;
        assert_eq!(schedule.upcoming(Some(now)), Some(expected));
    }

    #[test]
    fn multiple_schedules_should_work() {
        let now = Utc::now();
        let shortest = TimeDelta::days(2);
        let longest = TimeDelta::days(5);

        let schedule = TaskTrigger::Multiple(vec![
            TaskTrigger::Interval(shortest),
            TaskTrigger::Interval(longest),
        ]);

        let expected = now + shortest;
        assert_eq!(schedule.upcoming(Some(now)), Some(expected));
    }
}
