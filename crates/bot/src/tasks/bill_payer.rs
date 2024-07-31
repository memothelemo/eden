use async_trait::async_trait;
use eden_tasks::{Task, TaskPerformInfo, TaskResult};
use eden_utils::Result;
use serde::{Deserialize, Serialize};

use crate::Bot;

#[derive(Debug, Deserialize, Serialize)]
pub struct BillPayer;

#[async_trait]
impl Task for BillPayer {
    type State = Bot;

    fn task_type() -> &'static str
    where
        Self: Sized,
    {
        "payer::bill"
    }

    async fn perform(&self, _info: &TaskPerformInfo, _state: Self::State) -> Result<TaskResult> {
        todo!()
    }
}
