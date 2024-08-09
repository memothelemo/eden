use eden_tasks::prelude::*;
use eden_utils::{error::exts::ResultExt, Result};
use serde::{Deserialize, Serialize};

use crate::{errors::RegisterCommandsError, BotRef};

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterCommands;

#[async_trait]
impl Task for RegisterCommands {
    type State = BotRef;

    async fn perform(&self, _ctx: &TaskRunContext, bot: Self::State) -> Result<TaskResult> {
        let bot = bot.get();
        crate::interactions::commands::register(&bot)
            .await
            .change_context(RegisterCommandsError)?;

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
