use eden_db::schema::TaskPriority;
use serde::de::DeserializeOwned;
use std::sync::Arc;

use crate::queue::periodic::PeriodicTask;
use crate::task::Task;

use super::Queue;

pub struct TaskRegistryMeta<S> {
    pub(crate) deserializer: DeserializerFn<S>,
    pub(crate) kind: &'static str,
    pub(crate) is_periodic: bool,
    pub(crate) priority: PriorityFn,
}

pub type DeserializerFn<S> = Box<
    dyn Fn(serde_json::Value) -> serde_json::Result<Box<dyn Task<State = S>>>
        + Send
        + Sync
        + 'static,
>;

pub type PriorityFn = Box<dyn Fn() -> TaskPriority + Send + Sync + 'static>;

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

    #[allow(clippy::unwrap_used)]
    #[must_use]
    pub fn register_task<T>(self) -> Self
    where
        T: Task<State = S> + DeserializeOwned,
    {
        // assume it was if it is locked
        let is_running = self
            .0
            .runner_handle
            .try_lock()
            .map(|v| v.is_some())
            .unwrap_or(true);

        assert!(
            !is_running,
            "Registering task while the queue is running is not allowed!"
        );
        assert!(
            !self.is_task_registered::<T>(),
            "Task {:?} is already registered",
            T::task_type()
        );

        tracing::trace!("registered task {:?}", T::task_type());
        let deserializer: DeserializerFn<S> = Box::new(|value| {
            let task: T = serde_json::from_value(value)?;
            Ok(Box::new(task))
        });

        let metadata: TaskRegistryMeta<S> = TaskRegistryMeta {
            deserializer,
            kind: T::task_type(),
            is_periodic: T::schedule().is_periodic(),
            priority: Box::new(T::priority),
        };

        if T::schedule().is_periodic() {
            let mut tasks = self.0.periodic_tasks.try_write().unwrap();
            tasks.push(Arc::new(PeriodicTask::new::<_, T>()));
        }

        self.0.registry.insert(T::task_type(), metadata);
        self
    }
}
