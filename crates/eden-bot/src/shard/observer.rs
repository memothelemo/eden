use eden_settings::Settings;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{debug, info, trace, warn};
use twilight_gateway::{Shard, ShardId};

use super::manager::ShardManagerNotification;
use super::runner::{ShardHandle, ShardRunner, ShardRunnerMessage};
use super::ShardManager;
use crate::{flags, Bot};

/// Monitors all shards
#[allow(unused)]
pub struct ShardObserver {
    bot: Bot,
    manager: Arc<ShardManager>,
    rx: Receiver<ShardObserverMessage>,
    settings: Arc<Settings>,
    shards: Arc<Mutex<HashMap<ShardId, ShardHandle>>>,

    notify_tx: Sender<ShardNotification>,
    notify_rx: Receiver<ShardNotification>,
    manager_notify_tx: Sender<ShardManagerNotification>,

    connected_shards: Vec<ShardId>,
}

impl ShardObserver {
    #[must_use]
    pub fn new(
        bot: Bot,
        manager: Arc<ShardManager>,
        rx: Receiver<ShardObserverMessage>,
        manager_notify_tx: Sender<ShardManagerNotification>,
        shards: Arc<Mutex<HashMap<ShardId, ShardHandle>>>,
        settings: Arc<Settings>,
    ) -> Self {
        let (notify_tx, notify_rx) = mpsc::unbounded_channel();
        Self {
            bot,
            manager,
            rx,
            settings,
            shards,

            notify_tx,
            notify_rx,
            manager_notify_tx,

            connected_shards: Vec::new(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn run(mut self) {
        debug!("running shard observer");

        loop {
            trace!("loop iteration started");

            let now = Instant::now();
            tokio::select! {
                message = self.rx.recv() => {
                    if let Some(message) = message.as_ref() {
                        trace!("received observer message: {message:?}");
                    }
                    match message {
                        Some(ShardObserverMessage::StartShard(id)) => {
                            self.start(id).await;
                        }
                        Some(ShardObserverMessage::Shutdown) => {
                            self.shutdown_all(false).await;
                        }
                        Some(ShardObserverMessage::ShutdownShard(id)) => {
                            self.shutdown(id, false).await;
                        }
                        Some(ShardObserverMessage::Abort) => {
                            self.shutdown_all(true).await;
                        }
                        Some(ShardObserverMessage::Terminate) => {
                            self.shutdown_all(false).await;
                            break;
                        }
                        None => {
                            trace!("self.rx is closed");
                            break;
                        }
                    };
                },
                // observer notification
                message = self.notify_rx.recv() => {
                    if let Some(message) = message {
                        trace!(
                            ?message,
                            "received shard notification from shard {}",
                            message.shard_id()
                        );
                        self.handle_notification(message).await;
                    }
                }
            }

            let elapsed = now.elapsed();
            trace!(?elapsed, "loop iteration ended");
        }

        debug!("closing shard observer");
    }
}

impl ShardObserver {
    async fn start(&mut self, id: ShardId) {
        let token = self.settings.bot.token.expose().to_string();
        let config = twilight_gateway::Config::builder(token, flags::INTENTS)
            .event_types(flags::FILTERED_EVENT_TYPES)
            .queue(self.manager.queue.clone())
            .build();

        let shard = Shard::with_config(id, config);
        let (runner, handle) = ShardRunner::new(self.bot.clone(), self.notify_tx.clone(), shard);

        eden_utils::tokio::spawn("eden_bot::shard::runner::start", async move {
            runner.run().await;
        });
        self.shards.lock().await.insert(id, handle);
    }

    #[tracing::instrument(skip(self))]
    async fn shutdown(&mut self, id: ShardId, abort: bool) {
        let shards = self.shards.lock().await;
        let Some(handle) = shards.get(&id) else {
            warn!("could not shutdown shard {id} (missing handle)");
            return;
        };

        let message = if abort {
            ShardRunnerMessage::Abort
        } else {
            ShardRunnerMessage::Shutdown
        };

        let message_kind = message.kind();
        if let Err(error) = handle.runner_tx.send(message) {
            warn!(%error, "failed to {message_kind} shard {id} while trying to send message to shard runner");
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn shutdown_all(&mut self, abort: bool) {
        let shards = {
            let shards = self.shards.lock().await;
            if shards.is_empty() {
                return;
            }
            shards.keys().copied().collect::<Vec<_>>()
        };

        info!("shutting down {} shard(s)", shards.len());
        for id in shards {
            self.shutdown(id, abort).await;
        }
    }
}

impl ShardObserver {
    #[tracing::instrument(skip_all, level = "debug", fields(
        notification.kind = %value.kind(),
        notification.shard.id = %value.shard_id()
    ))]
    async fn handle_notification(&mut self, value: ShardNotification) {
        let total = self.shards.lock().await.len();
        let (should_log, increased, id) = match value {
            ShardNotification::Connected(id) => {
                self.connected_shards.push(id);
                (true, true, id)
            }
            ShardNotification::Restarting(id) => {
                eden_utils::vec::remove_if_exists(&mut self.connected_shards, &id);
                (true, false, id)
            }
            ShardNotification::Disconnected(id) => {
                eden_utils::vec::remove_if_exists(&mut self.connected_shards, &id);
                self.manager.remove_shard(id).await;
                (false, false, id)
            }
        };

        let connected = self.connected_shards.len();
        self.notify_shard_connections(connected, total);
        self.manager
            .connected
            .store(connected as u64, Ordering::Relaxed);

        if should_log {
            if increased {
                info!("{connected}/{total} shard(s) connected");
            } else {
                warn!("{connected}/{total} shard(s) connected (shard {id} got disconnected)");
            }
        }
    }

    fn notify_shard_connections(&self, connected: usize, total: usize) {
        let message = ShardManagerNotification::UpdatedShardConnections {
            connected: connected as u64,
            total: total as u64,
        };
        if let Err(error) = self.manager_notify_tx.send(message) {
            warn!(%error, "failed to send updated shard connections notification to the shard manager");
        }
    }
}

/// Messages that can be sent from the shard manager to the shard observer.
#[derive(Debug, Clone)]
#[allow(unused)]
pub enum ShardObserverMessage {
    /// Message to start a shard.
    StartShard(ShardId),
    /// Message to shutdown the shard observer and its shards.
    Shutdown,
    /// Message to shutdown a shard.
    ShutdownShard(ShardId),
    /// Message to abort the shard observer and its shards.
    Abort,
    /// Terminate and stop the shard observer loop.
    Terminate,
}

/// Messages that can be sent from a shard to shard observer and it is
/// used to notify the shard observer something with the shard.
#[derive(Debug)]
pub enum ShardNotification {
    /// A shard is ready or resumed and successfully connected to the
    /// gateway with an active session.
    Connected(ShardId),
    /// A shard is restarting the gateway connection.
    Restarting(ShardId),
    /// A shard is successfully disconnected the gateway connection.
    Disconnected(ShardId),
}

impl ShardNotification {
    /// Gets the type string of the notification.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Restarting(..) => "restarting",
            Self::Disconnected(..) => "disconnected",
            Self::Connected(..) => "connected",
        }
    }

    /// Gets the source shard ID from the notification.
    #[must_use]
    pub fn shard_id(&self) -> ShardId {
        match self {
            Self::Restarting(id) => *id,
            Self::Disconnected(id) => *id,
            Self::Connected(id) => *id,
        }
    }
}
