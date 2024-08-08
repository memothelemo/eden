use eden_tasks::prelude::*;
use eden_utils::Result;
use serde::{Deserialize, Serialize};

use crate::BotRef;

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterCommands;

#[async_trait]
impl Task for RegisterCommands {
    type State = BotRef;

    async fn perform(&self, _ctx: &TaskRunContext, _state: Self::State) -> Result<TaskResult> {
        Ok(TaskResult::Completed)
    }

    fn kind() -> &'static str {
        "eden::tasks::register_commands"
    }

    fn priority() -> TaskPriority {
        TaskPriority::High
    }

    fn temporary() -> bool {
        true
    }
}
