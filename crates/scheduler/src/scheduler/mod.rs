use dashmap::DashMap;
use eden_utils::Result;
use serde::Serialize;
use std::sync::Arc;

mod catch_unwind;
mod config;
mod error;
mod internal;
mod registry;
mod scheduled;

use crate::Task;

use self::config::{BuilderState, TaskSchedulerConfig};
use self::registry::TaskRegistryMeta;

pub use self::error::*;
pub use self::scheduled::*;

#[allow(private_interfaces)]
#[derive(Clone)]
pub struct TaskScheduler<S = BuilderState>(pub(crate) Arc<TaskSchedulerInternal<S>>);

impl<S> TaskScheduler<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub async fn clear_all(&self) -> Result<u64, ClearAllTasksError> {
        todo!()
    }
}

impl<S> TaskScheduler<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Attempts to queue a task with scheduled deadline is assigned
    /// depending on the interval period from the task specified and
    /// the current time of the system.
    ///
    /// If the returned value of task's [`schedule`](Task::schedule) function
    /// is [`TaskSchedule::None`](crate::TaskSchedule::None), it will throw an
    /// error.
    pub async fn queue<T>(&self, task: T) -> Result<(), QueueTaskError>
    where
        T: Task<State = S> + Serialize,
    {
        todo!()
    }

    /// Attempts to queue a task with a scheduled deadline
    pub async fn schedule<T>(&self, task: T, deadline: Scheduled) -> Result<(), QueueTaskError>
    where
        T: Task<State = S> + Serialize,
    {
        todo!()
        // let mut conn = self
        //     .0
        //     .pool
        //     .acquire()
        //     .await
        //     .change_context(QueueTaskError)
        //     .attach_printable("could not establish database connection")?;

        // self.try_queue_task(&mut conn, &task, Some(deadline))
        //     .await
        //     .attach_printable_lazy(|| format!("task.type: {}", T::kind()))
        //     .attach_printable_lazy(|| format!("task.data: {task:?}"))
    }
}

////////////////////////////////////////////////////////////////////////////
struct TaskSchedulerInternal<S> {
    config: TaskSchedulerConfig,
    pool: sqlx::PgPool,
    registry: Arc<DashMap<&'static str, TaskRegistryMeta<S>>>,
    state: S,
}

impl<S> std::fmt::Debug for TaskScheduler<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskRunner")
            .field("config", &self.0.config)
            .field("registered_tasks", &self.0.registry.len())
            .field("state", &std::any::type_name::<S>())
            .finish()
    }
}
