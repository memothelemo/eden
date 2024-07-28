use futures::future::select;
use tokio::sync::mpsc;
use twilight_gateway::ShardId;

#[derive(Debug)]
pub enum ShardObserverMessage {
    Restart(ShardId),
    Ready(ShardId),
}

#[allow(unused)]
#[tracing::instrument(skip_all, fields(shards.total = %total))]
pub async fn observe_shards(total: usize, mut rx: mpsc::UnboundedReceiver<ShardObserverMessage>) {
    use futures::future::Either::*;

    tracing::debug!("started shards observer thread");
    let mut active = 0;
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
        match message {
            ShardObserverMessage::Restart(id) => {
                active -= 1;
                #[cfg(not(release))]
                tracing::warn!(shard.id = %id, "{active}/{total} shard(s) connected");
                id
            }
            ShardObserverMessage::Ready(id) => {
                active += 1;
                #[cfg(not(release))]
                tracing::info!(shard.id = %id, "{active}/{total} shard(s) connected");
                id
            }
        };

        #[cfg(release)]
        {
            println!("{active}/{total} shard(s) connected");
        }
    }
}
