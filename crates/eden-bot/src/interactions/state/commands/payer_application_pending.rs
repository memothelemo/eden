use crate::interactions::state::{
    AnyStatefulCommand, CommandTriggerAction, StatefulCommandTrigger,
};
use crate::Bot;

#[derive(Debug)]
pub struct PayerApplicationPendingState {}

impl AnyStatefulCommand for PayerApplicationPendingState {
    #[tracing::instrument(skip(_bot))]
    async fn on_trigger(
        &self,
        _bot: &Bot,
        _trigger: StatefulCommandTrigger,
    ) -> eden_utils::Result<CommandTriggerAction> {
        Ok(CommandTriggerAction::Nothing)
    }
}
