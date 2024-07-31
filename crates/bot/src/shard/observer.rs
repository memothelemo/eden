#[allow(unused)]
use eden_tasks::Scheduled;
use futures::future::select;
use tokio::sync::mpsc;
use twilight_gateway::ShardId;

use crate::Bot;

#[derive(Debug)]
pub enum ShardObserverMessage {
    Restart(ShardId),
    Ready(ShardId),
}

#[allow(unused)]
#[tracing::instrument(skip_all, fields(shards.total = %total))]
pub async fn observe_shards(
    bot: Bot,
    total: usize,
    mut rx: mpsc::UnboundedReceiver<ShardObserverMessage>,
) {
    use futures::future::Either::*;
    tracing::debug!("spawned shards observer thread");

    let mut active: usize = 0;
    let mut has_registered_commands = false;
    loop {
        let shutdown = Box::pin(eden_utils::shutdown::graceful());
        let message = match select(shutdown, Box::pin(rx.recv())).await {
            Left(..) | Right((None, ..)) => {
                tracing::debug!("closing shards observer thread");
                break;
            }
            Right((Some(message), ..)) => message,
        };

        // Alert the user of how many shards needed to connect
        // in order to consider the bot as "ready to use".
        match &message {
            ShardObserverMessage::Restart(id) => {
                active = active.checked_sub(1).unwrap_or_default();
                #[cfg(not(release))]
                tracing::warn!(shard.id = %id, "{active}/{total} shard(s) connected");
            }
            ShardObserverMessage::Ready(id) => {
                active = active.checked_add(1).unwrap_or_default();
                #[cfg(not(release))]
                tracing::info!(shard.id = %id, "{active}/{total} shard(s) connected");
            }
        };

        #[cfg(release)]
        {
            println!("{active}/{total} shard(s) connected");
        }

        let should_register_commands =
            !has_registered_commands && matches!(message, ShardObserverMessage::Ready(..));

        if should_register_commands {
            todo!()
            // let result = crate::interaction::commands::register_commands(&bot).await;
            // let Err(error) = result else {
            //     continue;
            // };
            // tracing::warn!(%error, "failed to register slash commands");

            // // register commands for 5 minutes, maybe we're rate limited
            // let result = bot
            //     .queue
            //     .schedule(crate::tasks::RegisterCommands, Scheduled::in_minutes(5))
            //     .await;

            // if let Err(error) = result {
            //     let error = error.anonymize();
            //     tracing::warn!(%error, "failed to schedule to register commands for later");
            // }
        }
    }
}
