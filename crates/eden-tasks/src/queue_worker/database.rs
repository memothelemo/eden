use chrono::{DateTime, Utc};
use eden_tasks_schema::forms::{InsertTaskForm, UpdateTaskForm};
use eden_tasks_schema::types::{Task, TaskRawData, TaskStatus};
use eden_utils::{error::exts::*, sql::QueryError, Result};
use eden_utils::{Error, ErrorCategory};
use sqlx::{pool::PoolConnection, Transaction};
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use super::QueueWorker;
use crate::{error::*, Scheduled};

impl<S: Clone + Send + Sync + 'static> QueueWorker<S> {
    /// Attempts to clear all queued tasks from the database.
    ///
    /// If it fails, this operation will revert back before the
    /// deletion of all available tasks.
    ///
    /// It returns the total amount of tasks deleted from
    /// the database.
    #[allow(private_interfaces)]
    #[tracing::instrument(skip_all, fields(worker.id = %self.0.id))]
    pub async fn clear_all(&self) -> Result<u64, ClearAllTasksError> {
        info!("clearing all queued tasks");

        let mut conn = self
            .db_transaction()
            .await
            .change_context(ClearAllTasksError)
            .attach_lazy(|| tags::ClearAllWithStatusTag::none())?;

        let deleted = Task::delete_all(&mut conn)
            .await
            .change_context(ClearAllTasksError)
            .attach_lazy(|| tags::ClearAllWithStatusTag::none())?;

        conn.commit()
            .await
            .into_eden_error()
            .change_context(ClearAllTasksError)
            .attach_printable("could not commit database transaction")
            .attach_lazy(|| tags::ClearAllWithStatusTag::none())?;

        self.0.registry.unblock_all_recurring_tasks().await;
        Ok(deleted)
    }

    /// Attempts to clear all queued tasks from the database
    /// with given status only.
    ///
    /// If it fails, this operation will revert back before the
    /// deletion of all available tasks.
    ///
    /// It returns the total amount of tasks deleted from
    /// the database.
    #[allow(private_interfaces)]
    #[tracing::instrument(skip_all, fields(worker.id = %self.0.id))]
    pub async fn clear_all_with_status(
        &self,
        status: TaskStatus,
    ) -> Result<u64, ClearAllTasksError> {
        info!(?status, "clearing all queued tasks with status {status:?}");
        let tag = tags::ClearAllWithStatusTag::status(status);

        let mut conn = self
            .db_transaction()
            .await
            .change_context(ClearAllTasksError)
            .attach_lazy(|| tag)?;

        let deleted = Task::delete_all_with_status(&mut conn, status)
            .await
            .change_context(ClearAllTasksError)
            .attach_lazy(|| tag)?;

        conn.commit()
            .await
            .into_eden_error()
            .change_context(ClearAllTasksError)
            .attach_printable("could not commit database transaction")
            .attach_lazy(|| tag)?;

        Ok(deleted)
    }

    /// Attempts to delete a queued task from the database using
    /// the specified task id.
    ///
    /// It returns a boolean whether the specified task exists before deletion.
    #[tracing::instrument(skip_all, fields(worker.id = %self.0.id))]
    pub async fn delete_queued_task(&self, id: Uuid) -> Result<bool, DeleteTaskError> {
        info!("deleting task {id}");
        let tag = tags::DeleteTaskTag { id };

        let mut conn = self
            .db_connection()
            .await
            .change_context(DeleteTaskError)
            .attach_lazy(|| tag)?;

        let task = Task::delete(&mut conn, id)
            .await
            .change_context(DeleteTaskError)
            .attach_lazy(|| tag)?;

        // unblock if it is a periodic task
        if let Some(task) = task.as_ref() {
            let registry = &self.0.registry;
            registry.unblock_for_recurring_task(&task.data.kind).await;
        }

        Ok(task.is_some())
    }

    pub(crate) async fn clear_temporary_tasks(&self) -> Result<(), ClearTemporaryTasksError> {
        debug!("clearing temporary tasks");

        let mut total = 0;
        let temporary_tasks = self.0.registry.items().filter(|t| t.is_temporary);
        for entry in temporary_tasks {
            debug!("clearing {:?} tasks", entry.kind);

            let tag = tags::ClearAllWithStatusTag::task(entry.kind, entry.rust_name);

            let mut conn = self
                .db_transaction()
                .await
                .change_context(ClearTemporaryTasksError)
                .attach_lazy(|| tag)?;

            let deleted = Task::delete_all_with_type(&mut conn, &entry.kind)
                .await
                .change_context(ClearTemporaryTasksError)
                .attach_lazy(|| tag)?;

            conn.commit()
                .await
                .into_eden_error()
                .change_context(ClearTemporaryTasksError)
                .attach_printable("could not commit database transaction")
                .attach_lazy(|| tag)?;

            total += deleted;
        }

        debug!("removed {total} temporary tasks");
        Ok(())
    }

    pub(crate) async fn requeue_stalled_tasks(&self, now: DateTime<Utc>) -> Result<()> {
        let mut conn = self.db_connection().await?;
        let threshold = self.0.stalled_tasks_threshold;
        let amount = Task::requeue_stalled(&mut conn, self.id(), threshold, Some(now)).await?;
        if amount > 0 {
            warn!("requeued {amount} stalled task(s)");
        } else {
            trace!("requeud {amount} stalled task(s)");
        }
        Ok(())
    }

    pub(crate) async fn update_recurring_tasks_blacklist(
        &self,
    ) -> Result<(), UpdateTaskBlacklistError> {
        use eden_tasks_schema::types::Task;
        debug!("updating blacklist of recurring tasks");

        let registry = &self.0.registry;
        registry.unblock_all_recurring_tasks().await;

        let mut conn = self
            .db_transaction()
            .await
            .change_context(UpdateTaskBlacklistError)?;

        let mut stream = Task::get_all(self.0.id).periodic(true).build().size(50);
        while let Some(tasks) = stream
            .next(&mut conn)
            .await
            .anonymize_error()
            .change_context(UpdateTaskBlacklistError)?
        {
            for task in tasks {
                registry.block_for_recurring_task(&task.data.kind).await;
            }
        }

        conn.commit()
            .await
            .anonymize_error_into()
            .attach_printable("could not commit database transaction")
            .change_context(UpdateTaskBlacklistError)?;

        debug!("successfully updated blacklist of recurring tasks");
        Ok(())
    }
}

impl<S: Clone + Send + Sync + 'static> QueueWorker<S> {
    /// Unsafe version of [`Queue::schedule`] but any registered task
    /// (recurring or persistent) can be scheduled.
    #[allow(clippy::cast_lossless)]
    #[tracing::instrument(skip_all)]
    pub(crate) async fn queue(
        &self,
        id: Option<Uuid>,
        raw_data: TaskRawData,
        scheduled: Scheduled,
        now: Option<DateTime<Utc>>,
        attempts: u16,
    ) -> Result<Uuid, ScheduleTaskError> {
        // Checking if this specified task is registered in the registry
        let Some(registry_item) = self.0.registry.find_item(&raw_data.kind) else {
            return Err(Error::context(ErrorCategory::Unknown, ScheduleTaskError))
                .attach_printable(format!(
                    "task {:?} is not registered in the registry",
                    raw_data.kind
                ));
        };

        // Block this task from running it locally regardless if it
        // reaches the deadline (if it is a recurring task)
        let deadline = scheduled.timestamp(now);
        let priority = registry_item.priority;
        if registry_item.is_recurring {
            self.0
                .registry
                .block_for_recurring_task(&registry_item.kind)
                .await;
        }

        // not much data is lost when converted from u16 to i32
        let attempts = attempts as i32;
        let form = InsertTaskForm::builder()
            .id(id)
            .attempts(attempts)
            .data(raw_data)
            .deadline(deadline)
            .periodic(registry_item.is_recurring)
            .priority(priority)
            .build();

        let mut conn = self
            .db_connection()
            .await
            .change_context(ScheduleTaskError)?;

        let queued_task = Task::insert(&mut conn, form)
            .await
            .change_context(ScheduleTaskError)
            .attach_printable("could not insert task into the database")?;

        Ok(queued_task.id)
    }

    #[allow(clippy::cast_lossless)]
    #[tracing::instrument(skip_all)]
    pub(crate) async fn requeue(
        &self,
        id: Uuid,
        now: Option<DateTime<Utc>>,
        scheduled: Scheduled,
        attempts: u16,
    ) -> Result<(), ScheduleTaskError> {
        // not much data is lost when converted from u16 to i32
        let attempts = attempts as i32;
        let deadline = scheduled.timestamp(now);
        let form = UpdateTaskForm::builder()
            .attempts(Some(attempts + 1))
            .deadline(Some(deadline))
            .status(Some(TaskStatus::Queued))
            .build();

        let mut conn = self
            .db_connection()
            .await
            .change_context(ScheduleTaskError)?;

        Task::update(&mut conn, id, form)
            .await
            .change_context(ScheduleTaskError)?;

        Ok(())
    }
}

impl<S: Clone + Send + Sync + 'static> QueueWorker<S> {
    /// Tries to establish database connection
    ///
    /// Refer to [sqlx's `PoolConnection` object](PoolConnection) for more documentation
    /// and how it should be used.
    pub(crate) async fn db_connection(&self) -> Result<PoolConnection<sqlx::Postgres>, QueryError> {
        let pool = &self.0.pool;
        pool.acquire()
            .await
            .into_eden_error()
            .attach_printable("unable to establish connection to the database")
    }

    /// Tries to establish database transaction.
    ///
    /// Refer to [sqlx's Transaction object](Transaction) for more documentation
    /// and how it should be used.
    pub(crate) async fn db_transaction(
        &self,
    ) -> Result<Transaction<'_, sqlx::Postgres>, QueryError> {
        let pool = &self.0.pool;
        pool.begin()
            .await
            .into_eden_error()
            .attach_printable("unable to start transaction from the database")
    }
}
