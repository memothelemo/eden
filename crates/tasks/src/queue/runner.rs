use chrono::{DateTime, Utc};
use std::future::Future;

use super::Queue;

impl<S> Queue<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn run_periodic_tasks(&self, _now: DateTime<Utc>) {
        // let periodic_tasks = self.0.registry.iter().filter(|v| v.is_periodic);
        // for task in periodic_tasks {
        //     // Do not try to run if it is blocked
        //     if self.is_periodic_task_blocked(&task.kind) {
        //         continue;
        //     }

        //     let queue_tx = self.clone();
        //     let kind = task.kind;
        //     self.spawn_fut(async move {
        //         let Some(task) = queue_tx.0.registry.get(kind) else {
        //             return;
        //         };

        //         // Generate a fake JobRawData object so that a periodic task
        //         // can be deserialized unless if it has something wrong with the
        //         // user configuration and stuff...
        //         let fake_task_id = Uuid::new_v4();
        //         let perform_info = TaskPerformInfo {
        //             id: fake_task_id,
        //             created_at: now,
        //             failed_attempts: 0,
        //             last_retry: None,
        //             is_retrying: false,
        //         };
        //         let raw_data = TaskRawData {
        //             kind: task.kind.into(),
        //             inner: serde_json::Value::Null,
        //         };

        //         queue_tx.block_periodic_task(task.kind, BlockedReason::Running);
        //         if let Err(error) = queue_tx.perform_task(perform_info, raw_data, &task).await {
        //             let error = error.anonymize();
        //             eprintln!("task {fake_task_id} with type {kind:?}; task failed: {error}",);
        //         }
        //         queue_tx.unblock_periodic_task(task.kind);
        //     });
        // }
    }

    #[allow(unused)]
    fn spawn_fut<F>(&self, task: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.0.running_tasks.spawn(task);
    }
}

// the main loop for queue
pub async fn runner<S>(queue: Queue<S>)
where
    S: Clone + Send + Sync + 'static,
{
    let poll_interval = queue
        .0
        .config
        .poll_interval
        .to_std()
        .unwrap_or_else(|_| std::time::Duration::from_secs(10));

    loop {
        let now = Utc::now();
        queue.run_periodic_tasks(now);

        let poll = tokio::time::sleep(poll_interval);
        let shutdown = queue.0.shutdown.cancelled();

        tokio::select! {
            _ = poll => {},
            _ = shutdown => {
                eprintln!("requested shutdown");
                return;
            },
        }
    }
}
