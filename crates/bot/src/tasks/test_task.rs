use async_trait::async_trait;
use eden_tasks::{Task, TaskPerformInfo, TaskResult};
use eden_utils::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::Bot;

#[derive(Debug, Deserialize, Serialize)]
pub struct TestTask;

#[async_trait]
impl Task for TestTask {
    type State = Bot;

    fn task_type() -> &'static str
    where
        Self: Sized,
    {
        "test_task"
    }

    async fn perform(&self, _info: &TaskPerformInfo, _state: Self::State) -> Result<TaskResult> {
        tracing::info!("sleeping...");
        tokio::time::sleep(Duration::from_secs(10)).await;

        tracing::info!("good morning!");
        Ok(TaskResult::Completed)
    }
}
