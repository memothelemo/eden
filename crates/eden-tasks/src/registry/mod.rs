use chrono::{DateTime, Utc};
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use eden_tasks_schema::types::TaskPriority;
use serde::de::DeserializeOwned;
use std::any::type_name;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::{debug, info, trace, warn};

use crate::task::Task;

mod recurring;
pub use self::recurring::RecurringTask;

/// Responsible for keeping all registered metadata of tasks
pub struct TaskRegistry<S> {
    items: Arc<DashMap<String, RegistryItem<S>>>,
    // We want to keep it read-only so that we don't have to use
    // Mutex to access a list of registered recurring tasks.
    recurring_tasks: RwLock<Vec<Arc<RecurringTask>>>,
}

impl<S: Clone + Send + Sync + 'static> TaskRegistry<S> {
    pub fn new() -> Self {
        Self {
            items: Arc::new(DashMap::new()),
            recurring_tasks: RwLock::new(Vec::new()),
        }
    }

    #[allow(clippy::unwrap_used)]
    pub fn register_task<T: DeserializeOwned + Task<State = S>>(&self) {
        // This is to easily print the exact object type causing the
        // problem instead of printing off its type
        let kind = T::kind();
        let type_name = type_name::<T>();
        assert!(
            !self.is_task_registered::<T>(),
            "Task {type_name:?} ({kind}) is already registered",
        );
        trace!("registered task {type_name:?} ({kind})");

        let deserializer: DeserializerFn<S> = Box::new(|value| {
            let task: T = serde_json::from_value(value)?;
            Ok(Box::new(task))
        });

        let is_recurring = T::trigger().is_recurring();
        let item: RegistryItem<S> = RegistryItem {
            deserializer,
            kind,
            is_recurring,
            is_temporary: T::temporary(),
            priority: T::priority(),
            rust_name: type_name,
        };
        self.items.insert(kind.to_string(), item);

        if is_recurring {
            let task = RecurringTask::new::<S, T>();
            self.recurring_tasks.try_write().unwrap().push(task);
        }
    }

    #[must_use]
    pub fn find_item(&self, kind: &str) -> Option<Ref<'_, String, RegistryItem<S>>> {
        self.items.get(&kind.to_string())
    }

    #[must_use]
    pub fn items(&self) -> dashmap::iter::Iter<'_, String, RegistryItem<S>> {
        self.items.iter()
    }

    #[must_use]
    pub fn is_task_registered<T: Task<State = S>>(&self) -> bool {
        self.items.contains_key(T::kind())
    }
}

impl<S: Clone + Send + Sync + 'static> TaskRegistry<S> {
    pub async fn unblock_all_recurring_tasks(&self) {
        let tasks = self.recurring_tasks.read().await;
        if tasks.is_empty() {
            return;
        }

        debug!("unblocking all periodic tasks");
        for task in tasks.iter() {
            task.set_blocked(false).await;
        }
    }

    #[must_use]
    pub async fn find_recurring_task(&self, kind: &'static str) -> Arc<RecurringTask> {
        let tasks = self.recurring_tasks.write().await;
        match tasks.iter().find(|task| task.kind == kind) {
            Some(n) => n.clone(),
            None => panic!("cannot find recurring task for {kind:?}"),
        }
    }

    #[must_use]
    pub async fn get_recurring_task(&self, kind: &str) -> Option<Arc<RecurringTask>> {
        let tasks = self.recurring_tasks.write().await;
        match tasks.iter().find(|task| task.kind == kind) {
            Some(n) => Some(n.clone()),
            None => None,
        }
    }

    pub async fn block_for_recurring_task(&self, kind: &str) {
        let tasks = self.recurring_tasks.write().await;
        match tasks.iter().find(|task| task.kind == kind) {
            Some(n) => {
                info!("blocked recurring task {kind:?}");
                n.set_blocked(true).await;
            }
            None => {
                warn!("could find recurring task for {kind:?} to block");
            }
        }
    }

    pub async fn unblock_for_recurring_task(&self, kind: &str) {
        let tasks = self.recurring_tasks.write().await;
        match tasks.iter().find(|task| task.kind == kind) {
            Some(n) => {
                info!("unblocked recurring task {kind:?}");
                n.set_blocked(true).await;
            }
            None => {
                warn!("could find recurring task for {kind:?} to unblock");
            }
        }
    }

    pub(crate) async fn update_recurring_tasks_deadline(&self, now: Option<DateTime<Utc>>) {
        debug!(?now, "updating deadlines for recurring tasks");

        let now = now.unwrap_or_else(Utc::now);
        for task in self.recurring_tasks().await.iter() {
            task.update_deadline(now).await;
        }
    }

    pub async fn recurring_tasks(&self) -> RwLockReadGuard<'_, Vec<Arc<RecurringTask>>> {
        self.recurring_tasks.read().await
    }
}

impl<S: Clone + Send + Sync + 'static> Debug for TaskRegistry<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskRegistry")
            .field("items", &self.items.len())
            .finish()
    }
}

pub struct RegistryItem<S> {
    pub(crate) deserializer: DeserializerFn<S>,
    pub(crate) kind: &'static str,
    pub(crate) is_recurring: bool,
    pub(crate) is_temporary: bool,
    pub(crate) priority: TaskPriority,
    pub(crate) rust_name: &'static str,
}

pub type DeserializerFn<S> = Box<
    dyn Fn(serde_json::Value) -> serde_json::Result<Box<dyn Task<State = S>>>
        + Send
        + Sync
        + 'static,
>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::SampleRecurringTask;

    type TestRegistry = TaskRegistry<()>;

    #[test]
    #[should_panic]
    fn should_crash_if_registered_task_twice() {
        let registry = TestRegistry::new();
        registry.register_task::<SampleRecurringTask>();
        registry.register_task::<SampleRecurringTask>();
    }
}
