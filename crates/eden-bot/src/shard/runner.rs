use eden_utils::error::exts::{AnyErrorExt, ErrorExt};
use eden_utils::{Error, ErrorCategory};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tokio::sync::{Mutex, MutexGuard};
use tokio_util::task::TaskTracker;
use tracing::{debug, trace, warn, Instrument, Span};
use twilight_gateway::error::ReceiveMessageErrorType;
use twilight_gateway::{CloseFrame, ConnectionStatus, Event, EventType, Latency, Shard, ShardId};
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::gateway::payload::outgoing::UpdatePresence;
use twilight_model::gateway::presence::{Activity, Status};

use super::observer::ShardNotification;
use super::{PresenceData, ShardManager};
use crate::events::EventContext;
use crate::BotRef;

pub struct ShardRunner {
    bot: BotRef,
    // We need the handle to manipulate something
    // with `latency` and `status` fields.
    handle: ShardHandle,
    manager: Arc<ShardManager>,
    observer: Sender<ShardNotification>,
    runner_rx: Receiver<ShardRunnerMessage>,

    ///////////////////////////////////////////////
    id: ShardId,
    presence: UpdatePresencePayload,
    last_status: ConnectionStatus,
    shard: Shard,
    tasks: TaskTracker,
}

impl ShardRunner {
    #[must_use]
    pub fn new(
        bot: BotRef,
        manager: Arc<ShardManager>,
        observer: Sender<ShardNotification>,
        presence: Option<UpdatePresencePayload>,
        shard: Shard,
    ) -> (Self, ShardHandle) {
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
            manager,
            runner_rx: rx,
            tasks: TaskTracker::new(),

            id: shard.id(),
            last_status: shard.status().clone(),
            presence: presence.unwrap_or_else(|| PresenceData::default().into()),
            shard,
        };

        (runner, handle)
    }

    #[tracing::instrument(skip_all, fields(shard.id = %self.shard.id()))]
    pub async fn run(mut self) {
        debug!("starting shard {}", self.shard.id());
        loop {
            let mut handle_latency = self.handle.latency.lock().await;
            *handle_latency = self.shard.latency().clone();
            drop(handle_latency);

            let status = self.shard.status().clone();
            if status != self.last_status {
                let mut handle_status = self.handle.status.lock().await;
                *handle_status = status.clone();
                self.handle_new_status(&status).await;
                self.last_status = status;
            }

            let action = self.next_action().await;
            let event = match action {
                ShardAction::Shutdown(ShutdownReason::Graceful) => {
                    self.shutdown(true).await;
                    return;
                }
                ShardAction::Shutdown(ShutdownReason::Abort) => {
                    self.shutdown(false).await;
                    return;
                }
                ShardAction::Shutdown(ShutdownReason::FatalError(error)) => {
                    self.shutdown(true).await;
                    if let Err(error) = self
                        .observer
                        .send(ShardNotification::FatalError(self.id, error))
                    {
                        warn!(%error, "could not notify shard observer that the shard {} got a fatal error", self.id);
                    }
                    return;
                }
                ShardAction::Continue => continue,
                ShardAction::UpdatePresence => {
                    self.update_presence().await;
                    continue;
                }
                ShardAction::NewEvent(event) => event,
            };

            let bot = self.bot.get();
            if matches!(event.kind(), EventType::Ready | EventType::Resumed) {
                debug!("shard {} is ready", self.id);
                if let Err(error) = self.observer.send(ShardNotification::Connected(self.id)) {
                    warn!(%error, "could not notify shard observer that the shard {} is connected to the gateway", self.id);
                }
                // update their presence while it is ready
                self.update_presence().await;
            }

            if let Event::Ready(data) = &event {
                bot.override_application_id(data.application.id);
            }
            trace!("received event {:?}", event.kind());

            let span = Span::current();
            let ctx = EventContext {
                bot,
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
                    self.manager.shutdown_all();

                    let source = Error::any(ErrorCategory::Unknown, source)
                        .change_context(GatewayFatalError)
                        .attach_printable(format!("with shard id: {}", self.id))
                        .anonymize();

                    ShardAction::Shutdown(ShutdownReason::FatalError(source))
                } else {
                    ShardAction::Continue
                }
            }
            Right((Some(ShardRunnerMessage::Abort), ..)) => {
                ShardAction::Shutdown(ShutdownReason::Abort)
            }
            Right((Some(ShardRunnerMessage::Shutdown), ..)) => {
                ShardAction::Shutdown(ShutdownReason::Graceful)
            }
            Right((Some(ShardRunnerMessage::SetActivites(activities)), ..)) => {
                self.presence.activities = activities;
                ShardAction::UpdatePresence
            }
            Right((Some(ShardRunnerMessage::SetPresence(presence)), ..)) => {
                self.presence.activities = presence.activities;
                self.presence.afk = presence.afk;
                self.presence.since = presence.since.map(|v| v.timestamp_millis() as u64);
                self.presence.status = presence.status;
                ShardAction::UpdatePresence
            }
            Right((Some(ShardRunnerMessage::SetStatus(status)), ..)) => {
                self.presence.status = status;
                ShardAction::UpdatePresence
            }
            Right((None, ..)) => {
                warn!("self.runner_rx is dropped. closing shard");
                ShardAction::Shutdown(ShutdownReason::Graceful)
            }
        }
    }

    async fn close_shard(&mut self) {
        // Don't need for absolutely shutdown the WebSocket connection if a shard
        // is fatally closed its connection to the Discord gateway
        if self.shard.status().is_fatally_closed() {
            return;
        }

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
        debug!("updating presence");

        let payload = UpdatePresence {
            d: self.presence.clone(),
            op: twilight_model::gateway::OpCode::PresenceUpdate,
        };
        if let Err(error) = self.shard.command(&payload).await {
            warn!(%error, "failed to update bot's presence");
        }
    }
}

macro_rules! log_shard_error {
    ($source:expr) => {
        if $source.is_fatal() {
            tracing::error!(error = %$source, "got shard fatal error");
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
    status: Arc<Mutex<ConnectionStatus>>,

    pub(super) latency: Arc<Mutex<Latency>>,
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

#[derive(Debug, Error)]
#[error("could not successfully connect to the gateway")]
struct GatewayFatalError;

/// What will [`ShardRunner`] do after the next WebSocket
/// message is processed.
#[derive(Debug)]
enum ShardAction {
    /// Continue the loop as usual
    Continue,
    /// The inner value determines if it should close gracefully.
    Shutdown(ShutdownReason),
    /// The shard should handle a new received event from the Discord gateway.
    NewEvent(Event),
    /// The shard must update the bot's presence.
    UpdatePresence,
}

/// Reasons why shutdown needs to be done
#[derive(Debug)]
enum ShutdownReason {
    /// A shard got a fatal error
    FatalError(eden_utils::Error),
    Graceful,
    Abort,
}
