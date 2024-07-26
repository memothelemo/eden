use crate::task::{Task, TaskSchedule};
use eden_db::schema::TaskPriority;
use serde::de::DeserializeOwned;

use super::Queue;

pub struct TaskRegistryMeta<S> {
    pub(crate) deserializer: DeserializerFn<S>,
    pub(crate) kind: &'static str,
    pub(crate) is_periodic: bool,
    pub(crate) priority: PriorityFn,
    pub(crate) schedule: ScheduleFn,
}

pub type DeserializerFn<S> = Box<
    dyn Fn(serde_json::Value) -> serde_json::Result<Box<dyn Task<State = S>>>
        + Send
        + Sync
        + 'static,
>;

pub type PriorityFn = Box<dyn Fn() -> TaskPriority + Send + Sync + 'static>;
pub type ScheduleFn = Box<dyn Fn() -> TaskSchedule + Send + Sync + 'static>;

impl<S> Queue<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub(super) fn is_task_registered<T>(&self) -> bool
    where
        T: Task<State = S>,
    {
        self.0.registry.contains_key(T::task_type())
    }

    #[must_use]
    pub fn register_task<T>(self) -> Self
    where
        T: Task<State = S> + DeserializeOwned,
    {
        assert!(
            !self.is_task_registered::<T>(),
            "Task {:?} is already registered",
            T::task_type()
        );

        let deserializer: DeserializerFn<S> = Box::new(|value| {
            let task: T = serde_json::from_value(value)?;
            Ok(Box::new(task))
        });

        let metadata: TaskRegistryMeta<S> = TaskRegistryMeta {
            deserializer,
            kind: T::task_type(),
            is_periodic: T::schedule().is_periodic(),
            priority: Box::new(T::priority),
            schedule: Box::new(T::schedule),
        };

        self.0.registry.insert(T::task_type(), metadata);
        self
    }
}
