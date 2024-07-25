use chrono::Utc;
use eden_db::forms::InsertTaskForm;
use eden_db::schema::{Task as TaskSchema, TaskRawData};
use eden_utils::{error::ResultExt, Result};
use eden_utils::{Error, ErrorCategory};
use serde::Serialize;

use super::error::*;
use super::{Scheduled, TaskScheduler};
use crate::Task;

impl<S> TaskScheduler<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub(crate) async fn try_queue_task<T>(
        &self,
        conn: &mut sqlx::PgConnection,
        task: &T,
        scheduled: Option<Scheduled>,
    ) -> Result<(), QueueTaskError>
    where
        T: Task<State = S> + Serialize,
    {
        todo!()
    }
}
