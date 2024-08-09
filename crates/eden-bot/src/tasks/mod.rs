use crate::context::BotQueue;

mod alert_payment;
mod clear_inactive_interaction_states;
mod register_commands;
mod setup_local_guild;

pub use self::alert_payment::*;
pub use self::clear_inactive_interaction_states::*;
pub use self::register_commands::*;
pub use self::setup_local_guild::*;

#[must_use]
pub(crate) fn register_all_tasks(queue: BotQueue) -> BotQueue {
    queue
        .register_task::<AlertPayment>()
        .register_task::<ClearInactiveInteractionStates>()
        .register_task::<RegisterCommands>()
        .register_task::<SetupLocalGuild>()
}
