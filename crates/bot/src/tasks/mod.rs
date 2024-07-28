mod bill_payer;
mod test_task;

pub use self::bill_payer::BillPayer;
pub use self::test_task::TestTask;

use eden_tasks::Queue;

pub(crate) fn register_all_tasks(queue: Queue<crate::Bot>) -> Queue<crate::Bot> {
    queue
        .register_task::<BillPayer>()
        .register_task::<TestTask>()
}
