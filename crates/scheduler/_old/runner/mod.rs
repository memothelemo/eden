use chrono::Utc;
use dashmap::DashMap;
use eden_db::forms::UpdateTaskForm;
use eden_db::schema::Task as TaskSchema;
use eden_db::schema::TaskStatus;
use eden_utils::error::AnyResultExt;
use eden_utils::error::ResultExt;
use eden_utils::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

mod catch_unwind;
mod config;
mod error;
mod internal;
mod schedule;

use self::internal::*;
use crate::Task;
use crate::TaskResult;

pub use self::config::*;
pub use self::error::*;
pub use self::schedule::*;

#[allow(private_interfaces)]
#[derive(Clone)]
pub struct TaskRunner<S = BuilderState>(pub(crate) Arc<TaskRunnerInner<S>>);

struct TaskRunnerInner<S> {
    config: TaskRunnerConfig,
    registry: Arc<DashMap<&'static str, TaskRegistryMeta<S>>>,
    pool: sqlx::PgPool,
    state: S,
}

impl<S> TaskRunner<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub async fn clear_all(&self) -> Result<u64, ClearAllTasksError> {
        internal::clear_all_queued_tasks(self).await
    }

    pub async fn queue_failed_tasks(&self) -> Result<(), QueueFailedTasksError> {
        let mut conn = self
            .transaction()
            .await
            .transform_context(QueueFailedTasksError)
            .attach_printable("could not start database transaction")?;

        let mut queue = TaskSchema::get_all().status(TaskStatus::Failed).build();
        let now = Utc::now();

        while let Some(tasks) = queue
            .next(&mut conn)
            .await
            .change_context(QueueFailedTasksError)
            .attach_printable("could not pull failed tasks")?
        {
            for task in tasks {
                let form = UpdateTaskForm::builder()
                    .status(Some(TaskStatus::Failed))
                    .build();

                internal::provide_task_data_if_error::<_, _, S>(
                    &task,
                    None,
                    now,
                    None,
                    TaskSchema::update(&mut conn, task.id, form)
                        .await
                        .change_context(QueueFailedTasksError)
                        .attach_printable("could not update status of a failed task"),
                )?;
            }
        }

        conn.commit()
            .await
            .change_context(QueueFailedTasksError)
            .attach_printable("could not commit database transaction")?;

        Ok(())
    }

    pub async fn process_routine_tasks(&self) -> Result<(), ProcessRoutineTasksError> {
        todo!()
    }

    pub async fn process_queued_tasks(&self) -> Result<(), ProcessQueuedTasksError> {
        let mut conn = self
            .transaction()
            .await
            .transform_context(ProcessQueuedTasksError)
            .attach_printable("could not start database transaction")?;

        // There are 32 bits to work for this value.
        let max_failed_attempts = self.0.config.max_failed_attempts as i64;
        let now = Utc::now();

        let mut queue = TaskSchema::pull_all_pending(max_failed_attempts, Some(now)).size(50);
        while let Some(tasks) = queue
            .next(&mut conn)
            .await
            .change_context(ProcessQueuedTasksError)
            .attach_printable("could not pull tasks")?
        {
            println!("pulled {} tasks", tasks.len());

            for task in tasks {
                let result = internal::provide_task_data_if_error::<_, _, S>(
                    &task,
                    None,
                    now,
                    None,
                    self.try_run_unknown_task(&mut conn, &task).await,
                )
                .attach_printable("could not run task");

                if let Err(error) = result {
                    eprintln!("Could not run task: {}", error.anonymize());
                }
            }
        }

        conn.commit()
            .await
            .change_context(ProcessQueuedTasksError)
            .attach_printable("could not commit database transaction")?;

        Ok(())
    }

    pub async fn push<J>(&self, task: J) -> Result<(), QueueTaskError>
    where
        J: Task<State = S> + Serialize,
    {
        self.queue_task(&task, None)
            .await
            .attach_printable_lazy(|| format!("task.type: {}", J::kind()))
            .attach_printable_lazy(|| format!("task.data: {task:?}"))
    }

    pub async fn schedule<J>(&self, task: J, schedule: Schedule) -> Result<(), QueueTaskError>
    where
        J: Task<State = S> + Serialize,
    {
        self.queue_task(&task, Some(schedule))
            .await
            .attach_printable_lazy(|| format!("id: {}", J::kind()))
            .attach_printable_lazy(|| format!("data: {task:?}"))
    }

    pub fn register_task<J>(self) -> Self
    where
        J: Task<State = S> + DeserializeOwned,
    {
        if self.0.registry.contains_key(J::kind()) {
            panic!("Task {:?} is already registered", J::kind());
        }

        let deserializer: DeserializerFn<S> = Box::new(|value| {
            let task: J = serde_json::from_value(value)?;
            Ok(Box::new(task))
        });

        let metadata: TaskRegistryMeta<S> = TaskRegistryMeta {
            deserializer,
            kind: J::kind(),
            schedule: Box::new(J::schedule),
        };

        self.0.registry.insert(J::kind(), metadata);
        self
    }
}

impl<S> TaskRunner<S>
where
    S: Clone + Send + Sync + 'static,
{
    async fn transaction(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
        self.0.pool.begin().await.anonymize_error()
    }

    async fn try_run_unknown_task(
        &self,
        conn: &mut sqlx::PgConnection,
        schema: &TaskSchema,
    ) -> Result<(), RunTaskError> {
        // Search for that type of task from the registry
        let kind = schema.data.kind.as_str();
        let Some(registry_meta) = self.0.registry.get(kind) else {
            return Err(eden_utils::Error::context(
                eden_utils::ErrorCategory::Unknown,
                RunTaskError,
            ))
            .attach_printable(format!("unknown task {kind:?} (not registered in registry)"));
        };

        let deserializer = &*registry_meta.deserializer;
        let task = deserializer(schema.data.data.clone())
            .map_err(|e| eden_utils::Error::any(eden_utils::ErrorCategory::Unknown, e))
            .transform_context(RunTaskError)
            .attach_printable_lazy(|| {
                format!("could not deserialize task {:?}", registry_meta.kind)
            })?;

        println!(
            "running task {} with type {:?}; data = {task:?}",
            schema.id, registry_meta.kind
        );

        match internal::run_task(self, &*task, &registry_meta).await {
            Ok(new_status) => match new_status {
                TaskResult::Completed => {
                    println!("completed");
                }
                TaskResult::Fail(_) => {
                    println!("failed");
                }
                TaskResult::RetryIn(_) => {
                    println!("retry");
                }
            },
            Err(error) => {
                TaskSchema::fail(conn, schema.id)
                    .await
                    .change_context(RunTaskError)
                    .attach_printable("could not fail task")?;

                return Err(error);
            }
        }

        Ok(())
    }

    async fn queue_task<J>(&self, task: &J, schedule: Option<Schedule>) -> Result<(), QueueTaskError>
    where
        J: Task<State = S> + Serialize,
    {
        // checking if this specified task is registered in the registry
        if !self.0.registry.contains_key(J::kind()) {
            return Err(eden_utils::Error::context(
                eden_utils::ErrorCategory::Unknown,
                QueueTaskError,
            ))
            .attach_printable(format!(
                "task {:?} is not registered in the registry",
                J::kind()
            ));
        }

        // make sure that task (with schedule is set to None) has a
        // periodic schedule (retrieved from `J::schedule().is_periodic()`)
        if schedule.is_none() && !J::schedule().is_periodic() {
            return Err(eden_utils::Error::context(
                eden_utils::ErrorCategory::Unknown,
                QueueTaskError,
            ))
            .attach_printable(format!(
                "task {:?} is not periodic, consider putting schedule",
                J::kind()
            ));
        }

        internal::insert_into_queue_db(self, task, schedule)
            .await
            .attach_printable("could not queue task into the database")
    }
}

impl<S> std::fmt::Debug for TaskRunner<S>
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

impl TaskRunner<BuilderState> {
    #[must_use]
    pub const fn builder() -> TaskRunnerConfig {
        TaskRunnerConfig::new()
    }
}
