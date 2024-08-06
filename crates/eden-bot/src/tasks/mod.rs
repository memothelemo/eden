use eden_tasks::QueueWorker;

#[must_use]
pub(crate) fn register_all_tasks<S>(queue: QueueWorker<S>) -> QueueWorker<S>
where
    S: Clone + Send + Sync + 'static,
{
    queue
}
