mod context;
mod observer;

pub use self::context::*;
pub use self::observer::*;

use crate::Bot;
use tokio::sync::mpsc;
use tokio_util::task::TaskTracker;
use twilight_gateway::error::ReceiveMessageErrorType;
use twilight_gateway::EventType;
use twilight_gateway::{CloseFrame, Event, Shard};

enum ShardAction {
    Ignore,
    NewEvent(Event),
    Close,
}

#[tracing::instrument(skip_all, fields(shard.id = %shard.id()))]
pub async fn main(
    mut shard: Shard,
    bot: Bot,
    observer_tx: mpsc::UnboundedSender<ShardObserverMessage>,
) {
    let id = shard.id();
    tracing::debug!("starting shard {id}");

    let mut did_send_disconnect_msg = true;
    let tasks = TaskTracker::new();

    loop {
        if shard.status().is_disconnected() && !did_send_disconnect_msg {
            did_send_disconnect_msg = true;
            if let Err(error) = observer_tx.send(ShardObserverMessage::Restart(id)) {
                tracing::warn!(%error, "cannot send message to shards observer");
            }
        }

        let action = get_shard_action(&mut shard).await;
        let event = match action {
            ShardAction::Close => break,
            ShardAction::Ignore => continue,
            ShardAction::NewEvent(e) => e,
        };

        let ctx = ShardContext {
            bot: bot.clone(),
            latency: shard.latency().clone(),
            id,
        };

        // ready/resumed event will happen once per successful connection anyway :)
        if matches!(event.kind(), EventType::Ready | EventType::Resumed) {
            did_send_disconnect_msg = false;
            if let Err(error) = observer_tx.send(ShardObserverMessage::Ready(id)) {
                tracing::warn!(%error, "cannot send message to shards observer");
            }
        }

        tracing::trace!("received event {:?}", event.kind());
        bot.cache.update(&event);
        tasks.spawn(crate::events::handle_event(ctx, event));
    }

    tracing::info!("closing shard {id}");
    tasks.close();

    // wait for shard tasks to be complete
    if !tasks.is_empty() {
        tracing::info!("waiting for {} event(s) to process", tasks.len());
        tokio::select! {
            _ = tasks.wait() => {
                tracing::info!("all event(s) are processed");
            },
            _ = eden_utils::shutdown::aborted() => {
                tracing::warn!("aborting all event process(es)");
            }
        }
    }

    tokio::select! {
        _ = try_close_shard(&mut shard) => {},
        _ = eden_utils::shutdown::aborted() => {}
    }
}

macro_rules! log_shard_error {
    ($source:expr) => {
        if $source.is_fatal() {
            tracing::error!(error = %$source, "got fatal shard error");
        } else {
            tracing::warn!(error = %$source, "got shard error");
        }
    };
}

async fn try_close_shard(shard: &mut Shard) {
    if let Err(error) = shard.close(CloseFrame::NORMAL).await {
        tracing::warn!(%error, "failed to close shard connection");
    }

    // Wait until WebSocket connection is FINALLY CLOSED
    loop {
        match shard.next_message().await {
            Ok(..) => {}
            // Interesting error while I was hosting my own bot,
            // you can disable this if you really want to.
            Err(source) if matches!(source.kind(), ReceiveMessageErrorType::Io) => {}
            Err(source) => {
                log_shard_error!(source);
            }
        }
        break;
    }
}

async fn get_shard_action(shard: &mut Shard) -> ShardAction {
    use futures::future::{select, Either::*};

    let next_event = Box::pin(shard.next_event());
    let shutdown = Box::pin(eden_utils::shutdown::graceful());
    match select(next_event, shutdown).await {
        Left((Ok(event), ..)) => ShardAction::NewEvent(event),
        Left((Err(source), ..)) => {
            log_shard_error!(source);
            if source.is_fatal() {
                tracing::warn!("shutting down Eden because of this error");
                eden_utils::shutdown::shutdown();
                ShardAction::Close
            } else {
                ShardAction::Ignore
            }
        }
        Right(..) => ShardAction::Close,
    }
}
