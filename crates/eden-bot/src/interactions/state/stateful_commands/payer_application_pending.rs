use crate::interactions::{StatefulCommand, StatefulCommandResult, StatefulCommandTrigger};
use crate::Bot;
use eden_utils::Result;

#[derive(Debug)]
pub struct PayerApplicationPending {}

impl StatefulCommand for PayerApplicationPending {
    #[tracing::instrument(skip(self))]
    async fn on_trigger(
        &self,
        bot: &Bot,
        _trigger: StatefulCommandTrigger,
    ) -> Result<StatefulCommandResult> {
        Ok(StatefulCommandResult::Ignore)
    }
}
