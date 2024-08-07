use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tokio::sync::Mutex;
use tracing::{debug, info, trace};
use twilight_gateway::queue::{LocalQueue, Queue};
use twilight_gateway::ShardId;

use crate::Bot;

use super::observer::{ShardObserver, ShardObserverMessage};
use super::ShardHandle;

#[derive(Debug)]
pub struct ShardManager {
    pub(crate) connected: AtomicU64,
    pub(crate) queue: Arc<dyn Queue>,

    observer: Sender<ShardObserverMessage>,
    notify_rx: Arc<Mutex<Receiver<ShardManagerNotification>>>,
    shards: Arc<Mutex<HashMap<ShardId, ShardHandle>>>,

    /// First shard to initialize.
    first: AtomicU64,
    /// Number of shards to initialize.
    size: AtomicU64,
    /// Total shards needed to be utilized for the bot.
    total: AtomicU64,
}

impl ShardManager {
    #[must_use]
    pub fn new(bot: Bot) -> Arc<Self> {
        let (observer_tx, observer_rx) = mpsc::unbounded_channel();
        let (notify_tx, notify_rx) = mpsc::unbounded_channel();
        let notify_rx = Arc::new(Mutex::new(notify_rx));

        let shards = Arc::new(Mutex::new(HashMap::new()));
        let settings = bot.settings.clone();

        let manager = Arc::new(Self {
            connected: AtomicU64::new(0),
            queue: Arc::new(LocalQueue::new()),

            observer: observer_tx,
            notify_rx,
            shards: shards.clone(),

            first: AtomicU64::new(settings.bot.sharding.first()),
            size: AtomicU64::new(settings.bot.sharding.size()),
            total: AtomicU64::new(settings.bot.sharding.total()),
        });

        let observer = ShardObserver::new(
            bot,
            manager.clone(),
            observer_rx,
            notify_tx,
            shards,
            settings,
        );

        eden_utils::tokio::spawn("eden_bot::shard::observer::run", async move {
            observer.run().await;
        });

        manager
    }

    /// Gets the total number of connected shards.
    #[must_use]
    pub fn connected(&self) -> u64 {
        self.connected.load(Ordering::Relaxed)
    }

    /// Gets the total number of shards needed to connect.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    /// Gets the [`ShardHandle`] from a given shard ID.
    ///
    /// Read more about [`ShardHandle`] to know the details of it.
    pub async fn shard(&self, id: ShardId) -> Option<ShardHandle> {
        self.shards.lock().await.get(&id).cloned()
    }

    /// Gets all [handles](ShardHandle) of all initialized shards.
    ///
    /// Read more about [`ShardHandle`] to know the details of it.
    pub async fn shards(&self) -> Vec<ShardHandle> {
        self.shards.lock().await.values().cloned().collect()
    }

    /// Gets all initialized shards by their [shard ID](ShardId).
    pub async fn initialized_shards(&self) -> Vec<ShardId> {
        self.shards.lock().await.keys().copied().collect()
    }
}

impl ShardManager {
    pub fn abort(&self) {
        drop(self.observer.send(ShardObserverMessage::Abort));
    }

    pub fn start_all(&self) {
        let shard_id_from = self.first.load(Ordering::Relaxed);
        let total = self.total.load(Ordering::Relaxed);
        let size = self.size.load(Ordering::Relaxed);

        let shard_id_to = shard_id_from + size;

        info!("starting {size} shard(s)");
        for id in shard_id_from..shard_id_to {
            // assuming they're checked from sharding settings
            let id = ShardId::new(id, total);
            self.boot_shard(id);
        }
    }

    pub fn shutdown_all(&self) {
        drop(self.observer.send(ShardObserverMessage::Shutdown));
    }

    /// Removes specific shard from the shard handlers map.
    pub(crate) async fn remove_shard(&self, id: ShardId) {
        debug!("removed shard {id} from the handler map");
        self.shards.lock().await.remove(&id);
    }

    fn boot_shard(&self, id: ShardId) {
        drop(self.observer.send(ShardObserverMessage::StartShard(id)));
    }
}

impl ShardManager {
    /// Waits for all shards to successfully connected to the Discord gateway.
    #[tracing::instrument(skip_all, level = "debug")]
    pub async fn wait_for_all_connected(&self) {
        let mut connected = self.connected();
        let mut total = self.total();

        // make sure the total is not zero though
        if connected == total && total > 0 {
            debug!("all {total} shard(s) are already connected. ignoring");
            return;
        }

        let mut notify = self.notify_rx.lock().await;
        debug!(
            "waiting for {connected}/{} shard(s) to be connected",
            total.clamp(1, total)
        );

        loop {
            if connected == total {
                break;
            }

            let notification = notify.recv().await;
            trace!(?notification, "received notification");

            match notification {
                Some(ShardManagerNotification::UpdatedShardConnections {
                    connected: updated_connected,
                    total: updated_total,
                }) => {
                    if connected != updated_connected || total != updated_total {
                        connected = updated_connected;
                        total = updated_total;
                        debug!("waiting for {connected}/{total} shard(s) to be connected");
                    }
                }
                None => {
                    debug!("self.notify_rx is dropped. closing loop");
                    break;
                }
            }
        }

        debug!("all shard(s) are connected");
    }

    /// Waits all for connected shards to successfully disconnected.
    #[tracing::instrument(skip_all, level = "debug")]
    pub async fn wait_for_all_closed(&self) {
        let connected = self.connected();
        if connected == 0 {
            debug!("already closed all shards. ignoring");
            return;
        }

        let total = self.total();
        let mut remaining = total - connected;

        info!("waiting for {remaining}/{total} shard(s) to be closed");

        let mut notify = self.notify_rx.lock().await;
        loop {
            if remaining == total {
                break;
            }

            let notification = notify.recv().await;
            trace!(?notification, "received notification");

            match notification {
                Some(ShardManagerNotification::UpdatedShardConnections {
                    connected: updated_connected,
                    ..
                }) => {
                    let updated_remaining = total - updated_connected;
                    if remaining != updated_remaining {
                        remaining = updated_remaining;
                        info!("waiting for {remaining}/{total} shard(s) to be closed");
                    }
                }
                None => {
                    debug!("self.notify_rx is dropped. closing loop");
                    break;
                }
            }
        }
        info!("all shard(s) are closed");
    }
}

impl Drop for ShardManager {
    fn drop(&mut self) {
        debug!("shard manager got dropped. terminating all shard services");
        drop(self.observer.send(ShardObserverMessage::Terminate));
    }
}

/// Messages that can be sent from shard observer to shard manager
/// and it is used to notify the shard manager about the connection
/// status of the all shards.
#[derive(Debug)]
pub(crate) enum ShardManagerNotification {
    UpdatedShardConnections { connected: u64, total: u64 },
}
