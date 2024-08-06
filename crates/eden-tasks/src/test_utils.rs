use crate::{TaskResult, TaskRunContext, TaskTrigger};

use async_trait::async_trait;
use chrono::TimeDelta;
use eden_utils::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct SampleRecurringTask;

#[async_trait]
impl crate::Task for SampleRecurringTask {
    type State = ();

    fn kind() -> &'static str
    where
        Self: Sized,
    {
        "eden_tasks::registry::SampleTask"
    }

    fn trigger() -> TaskTrigger
    where
        Self: Sized,
    {
        TaskTrigger::interval(TimeDelta::seconds(5))
    }

    async fn perform(&self, _ctx: &TaskRunContext, _state: Self::State) -> Result<TaskResult> {
        Ok(TaskResult::Completed)
    }
}
