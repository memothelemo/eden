mod bill_payer;
mod register_commands;

pub use self::bill_payer::BillPayer;
pub use self::register_commands::RegisterCommands;

use eden_tasks::Queue;

pub(crate) fn register_all_tasks(queue: Queue<crate::Bot>) -> Queue<crate::Bot> {
    queue
        .register_task::<BillPayer>()
        .register_task::<RegisterCommands>()
}
