use crate::task::{Task, TaskSchedule};
use serde::de::DeserializeOwned;

use super::TaskScheduler;

pub struct TaskRegistryMeta<S> {
    pub(crate) deserializer: DeserializerFn<S>,
    pub(crate) kind: &'static str,
    pub(crate) schedule: ScheduleFn,
}

pub type DeserializerFn<S> = Box<
    dyn Fn(serde_json::Value) -> serde_json::Result<Box<dyn Task<State = S>>>
        + Send
        + Sync
        + 'static,
>;

pub type ScheduleFn = Box<dyn Fn() -> TaskSchedule + Send + Sync + 'static>;

impl<S> TaskScheduler<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub(super) fn is_task_registered<T>(&self) -> bool
    where
        T: Task<State = S>,
    {
        self.0.registry.contains_key(T::kind())
    }

    #[must_use]
    pub fn register_task<T>(self) -> Self
    where
        T: Task<State = S> + DeserializeOwned,
    {
        if self.is_task_registered::<T>() {
            panic!("Task {:?} is already registered", T::kind());
        }

        let deserializer: DeserializerFn<S> = Box::new(|value| {
            let task: T = serde_json::from_value(value)?;
            Ok(Box::new(task))
        });

        let metadata: TaskRegistryMeta<S> = TaskRegistryMeta {
            deserializer,
            kind: T::kind(),
            schedule: Box::new(T::schedule),
        };

        self.0.registry.insert(T::kind(), metadata);
        self
    }
}
