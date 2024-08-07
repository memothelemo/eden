use crate::Bot;
use eden_tasks::QueueWorker;

#[must_use]
pub(crate) fn register_all_tasks(queue: QueueWorker<Bot>) -> QueueWorker<Bot> {
    queue
}
