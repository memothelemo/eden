use eden_tasks::prelude::*;
use eden_utils::Result;
use serde::{Deserialize, Serialize};

use crate::BotRef;

#[derive(Debug, Deserialize, Serialize)]
pub struct ClearInactiveInteractionStates;

#[async_trait]
impl Task for ClearInactiveInteractionStates {
    type State = BotRef;

    #[tracing::instrument(skip_all)]
    async fn perform(&self, _ctx: &TaskRunContext, state: Self::State) -> Result<TaskResult> {
        let bot = state.get();
        bot.command_state.clear_inactive().await;

        Ok(TaskResult::Completed)
    }

    fn trigger() -> TaskTrigger {
        TaskTrigger::interval(TimeDelta::seconds(30))
    }

    fn kind() -> &'static str {
        "eden::tasks::clear_inactive_interaction_states"
    }

    fn priority() -> TaskPriority {
        TaskPriority::High
    }
}
