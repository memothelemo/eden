use async_trait::async_trait;
use eden_tasks::{Task, TaskPerformInfo, TaskResult};
use eden_utils::Result;
use serde::{Deserialize, Serialize};

use crate::Bot;

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterCommands;

#[async_trait]
impl Task for RegisterCommands {
    type State = Bot;

    fn task_type() -> &'static str
    where
        Self: Sized,
    {
        "register_commands"
    }

    async fn perform(&self, _info: &TaskPerformInfo, _bot: Self::State) -> Result<TaskResult> {
        todo!()
    }
}
