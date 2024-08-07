use eden_utils::shutdown::ShutdownMode;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tokio::sync::{Mutex, MutexGuard};
use tokio_util::task::TaskTracker;
use tracing::{debug, trace, warn, Instrument, Span};
use twilight_gateway::error::ReceiveMessageErrorType;
use twilight_gateway::{CloseFrame, ConnectionStatus, Event, EventType, Latency, Shard, ShardId};
use twilight_model::gateway::presence::{Activity, Status};

use super::observer::ShardNotification;
use super::PresenceData;
use crate::events::EventContext;
use crate::Bot;

pub struct ShardRunner {
    bot: Bot,
    // We need the handle to manipulate something
    // with `latency` and `status` fields.
    handle: ShardHandle,
    observer: Sender<ShardNotification>,
    runner_rx: Receiver<ShardRunnerMessage>,

    ///////////////////////////////////////////////
    id: ShardId,
    presence: PresenceData,
    last_status: ConnectionStatus,
    shard: Shard,
    tasks: TaskTracker,
}

impl ShardRunner {
    #[must_use]
    pub fn new(bot: Bot, observer: Sender<ShardNotification>, shard: Shard) -> (Self, ShardHandle) {
        let (tx, rx) = mpsc::unbounded_channel();

        let handle = ShardHandle {
            id: shard.id(),
            latency: Arc::new(Mutex::new(shard.latency().clone())),
            runner_tx: tx,
            status: Arc::new(Mutex::new(shard.status().clone())),
        };

        let runner = Self {
            bot,
            handle: handle.clone(),
            observer,
            runner_rx: rx,
            tasks: TaskTracker::new(),

            id: shard.id(),
            last_status: shard.status().clone(),
            presence: PresenceData::default(),
            shard,
        };

        (runner, handle)
    }

    #[tracing::instrument(skip_all, level = "debug", fields(shard.id = %self.shard.id()))]
    pub async fn run(mut self) {
        debug!("starting shard {}", self.shard.id());
        loop {
            let status = self.shard.status().clone();
            if status != self.last_status {
                let mut handle_status = self.handle.status.lock().await;
                *handle_status = status.clone();
                self.handle_new_status(&status).await;
                self.last_status = status;
            }

            let action = self.next_action().await;
            let event = match action {
                ShardAction::Shutdown(graceful) => {
                    self.shutdown(graceful).await;
                    return;
                }
                ShardAction::Continue => continue,
                ShardAction::UpdatePresence => {
                    self.update_presence().await;
                    continue;
                }
                ShardAction::NewEvent(event) => event,
            };

            if matches!(event.kind(), EventType::Ready | EventType::Resumed) {
                debug!("shard {} is ready", self.id);
                if let Err(error) = self.observer.send(ShardNotification::Connected(self.id)) {
                    warn!(%error, "could not notify shard observer that the shard {} is connected to the gateway", self.id);
                }
            }
            trace!("received event {:?}", event.kind());

            let span = Span::current();
            let ctx = EventContext {
                bot: self.bot.clone(),
                latency: self.shard.latency().clone(),
                shard: self.handle.clone(),
            };
            self.tasks
                .spawn(crate::events::handle_event(ctx, event).instrument(span));
        }
    }

    async fn next_action(&mut self) -> ShardAction {
        use futures::future::{select, Either::*};

        let next_event = Box::pin(self.shard.next_event());
        let runner = Box::pin(self.runner_rx.recv());

        match select(next_event, runner).await {
            Left((Ok(event), ..)) => ShardAction::NewEvent(event),
            Left((Err(source), ..)) => {
                log_shard_error!(source);
                if source.is_fatal() {
                    eden_utils::shutdown::trigger(ShutdownMode::Graceful).await;
                    ShardAction::Shutdown(true)
                } else {
                    ShardAction::Continue
                }
            }
            Right((Some(ShardRunnerMessage::Abort), ..)) => ShardAction::Shutdown(false),
            Right((Some(ShardRunnerMessage::Shutdown), ..)) => ShardAction::Shutdown(true),
            Right((Some(ShardRunnerMessage::SetActivites(activities)), ..)) => {
                self.presence.activites = activities;
                ShardAction::UpdatePresence
            }
            Right((Some(ShardRunnerMessage::SetPresence(presence)), ..)) => {
                self.presence = presence;
                ShardAction::UpdatePresence
            }
            Right((Some(ShardRunnerMessage::SetStatus(status)), ..)) => {
                self.presence.status = status;
                ShardAction::UpdatePresence
            }
            Right((None, ..)) => {
                warn!("self.runner_rx is dropped. closing shard");
                ShardAction::Shutdown(true)
            }
        }
    }

    async fn close_shard(&mut self) {
        if let Err(error) = self.shard.close(CloseFrame::NORMAL).await {
            tracing::warn!(%error, "failed to close shard connection for {}", self.id);
        }

        // Wait until the shard's WebSocket connection is FINALLY CLOSED!
        loop {
            match self.shard.next_message().await {
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

    async fn shutdown(&mut self, graceful: bool) {
        if graceful {
            debug!("shutting down shard {}", self.id);
        } else {
            warn!("aborting shard {}", self.id);
        }
        self.tasks.close();

        // waiting for all running tasks to be completed.
        if !self.tasks.is_empty() && graceful {
            warn!(
                "waiting for {} event(s) to process for shard {}",
                self.tasks.len(),
                self.id
            );
            tokio::select! {
                _ = self.tasks.wait() => {
                    debug!("all event(s) from shard {} are processed", self.id);
                },
                _ = eden_utils::shutdown::aborted() => {}
            }
        }

        if graceful {
            tokio::select! {
                _ = self.close_shard() => {},
                _ = eden_utils::shutdown::aborted() => {}
            }
        }

        if let Err(error) = self.observer.send(ShardNotification::Disconnected(self.id)) {
            warn!(%error, "could not notify shard observer that the shard {} is disconnected", self.id);
        }
        debug!("shard {} is closed", self.id);
    }
}

impl ShardRunner {
    async fn handle_new_status(&self, status: &ConnectionStatus) {
        if !status.is_disconnected() {
            return;
        }

        if let Err(error) = self.observer.send(ShardNotification::Restarting(self.id)) {
            warn!(%error, "could not notify shard observer that the shard {} is restarting", self.id);
        }
    }

    async fn update_presence(&mut self) {
        // We're manually creating update presence since twilight
        // won't allow us to use `new` function without getting an error
        // if an empty set of activities is provided.
        let payload = self.presence.transform();
        if let Err(error) = self.shard.command(&payload).await {
            warn!(%error, "failed to update bot's presence");
        }
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
use log_shard_error;

/// A handle for a [`Shard`].
///
/// It allows to communicate with the shard without accessing the [`Shard`]
/// directly and perform shard-related actions like setting the presence or
/// shutting down in demand.
#[derive(Debug, Clone)]
pub struct ShardHandle {
    id: ShardId,
    latency: Arc<Mutex<Latency>>,
    status: Arc<Mutex<ConnectionStatus>>,
    pub(super) runner_tx: Sender<ShardRunnerMessage>,
}

impl ShardHandle {
    /// ID of an shard
    #[must_use]
    pub const fn id(&self) -> ShardId {
        self.id
    }

    /// Latency of the [`Shard`].
    #[must_use]
    pub async fn latency(&self) -> MutexGuard<'_, Latency> {
        self.latency.lock().await
    }

    /// Connection status of the [`Shard`].
    #[must_use]
    pub async fn status(&self) -> ConnectionStatus {
        self.status.lock().await.clone()
    }

    pub fn abort(&self) {
        self.send_to_shard(ShardRunnerMessage::Abort);
    }

    pub fn set_activities(&self, activities: impl Into<Option<Vec<Activity>>>) {
        let activities: Option<Vec<Activity>> = activities.into();
        self.send_to_shard(ShardRunnerMessage::SetActivites(
            activities.unwrap_or_default(),
        ));
    }

    pub fn set_presence(&self, presence: PresenceData) {
        self.send_to_shard(ShardRunnerMessage::SetPresence(presence));
    }

    pub fn set_status(&self, mut status: Status) {
        if status == Status::Offline {
            status = Status::Invisible;
        }
        self.send_to_shard(ShardRunnerMessage::SetStatus(status));
    }

    pub fn shutdown(&self) {
        self.send_to_shard(ShardRunnerMessage::Shutdown);
    }

    #[tracing::instrument(skip_all, fields(message = %message.kind()))]
    fn send_to_shard(&self, message: ShardRunnerMessage) {
        if let Err(e) = self.runner_tx.send(message) {
            warn!(error = %e, "failed to send inbound message to shard {}", self.id)
        }
    }
}

/// Messages that can be sent from the shard manager to a shard.
#[derive(Debug)]
pub enum ShardRunnerMessage {
    /// Indicates that this shard must abort immediately in the event
    /// of the user requested to abort the Eden bot process.
    Abort,
    // /// Indicates request to a shard to request to get guild members
    // RequestGuildMembers(ShardId, RequestGuildMembers),
    /// Indicates that shard needs to shutdown gracefully
    Shutdown,
    /// Indicates request to a shard to update their presence's activities.
    SetActivites(Vec<Activity>),
    /// Indicates request to a shard to update their presence entirely.
    SetPresence(PresenceData),
    /// Indicates request to a shard to change their presence status.
    SetStatus(Status),
}

impl ShardRunnerMessage {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Abort => "abort",
            Self::Shutdown => "shutdown",
            Self::SetActivites(..) => "set_activity",
            Self::SetPresence(..) => "set_presence",
            Self::SetStatus(..) => "set_status",
        }
    }
}

/// What will [`ShardRunner`] do after the next WebSocket
/// message is processed.
#[derive(Debug)]
enum ShardAction {
    /// Continue the loop as usual
    Continue,
    /// The inner value determines if it should close gracefully.
    Shutdown(bool),
    /// The shard should handle a new received event from the Discord gateway.
    NewEvent(Event),
    /// The shard must update the bot's presence.
    UpdatePresence,
}
