use async_trait::async_trait;
use eden_tasks::{Task, TaskPerformInfo, TaskResult};
use eden_utils::{error::AnyResultExt, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Bot;

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterCommands;

#[derive(Debug, Error)]
#[error("could not register commands")]
struct RegisterCommandsError;

#[async_trait]
impl Task for RegisterCommands {
    type State = Bot;

    async fn perform(&self, _info: &TaskPerformInfo, bot: Self::State) -> Result<TaskResult> {
        crate::interaction::cmds::register(&bot)
            .await
            .transform_context(RegisterCommandsError)?;

        Ok(TaskResult::Completed)
    }

    fn task_type() -> &'static str
    where
        Self: Sized,
    {
        "commands::register"
    }

    fn temporary() -> bool {
        true
    }
}
