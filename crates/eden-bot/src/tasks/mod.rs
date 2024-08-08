use crate::context::BotQueue;

mod register_commands;
mod setup_local_guild;

pub use self::register_commands::*;
pub use self::setup_local_guild::*;

#[must_use]
pub(crate) fn register_all_tasks(queue: BotQueue) -> BotQueue {
    queue.register_task::<RegisterCommands>()
}
