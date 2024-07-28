mod bill_payer;
pub use self::bill_payer::BillPayer;

use eden_tasks::Queue;

pub(crate) fn register_all_tasks(queue: Queue<crate::Bot>) -> Queue<crate::Bot> {
    queue.register_task::<BillPayer>()
}
