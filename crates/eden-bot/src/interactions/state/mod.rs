use chrono::{DateTime, TimeDelta, Utc};
use dashmap::DashMap;
use eden_settings::Settings;
use eden_utils::Result;
use std::fmt::Debug;
use std::sync::Arc;
use strum_macros::Display;
use tokio::sync::Mutex;
use tokio_util::task::TaskTracker;
use tracing::{debug, trace, warn, Span};
use twilight_model::id::marker::{ChannelMarker, InteractionMarker, MessageMarker};
use twilight_model::id::Id;

use crate::{Bot, BotRef};

pub mod commands;

/// Holds all states of all invoked command interactions in this bot.
///
/// It is also responsible for monitoring for any inactive command
/// interactions and running every possible command triggers that will
/// allow to continue the progress of the command.
///
/// Every command interaction state will be wiped after 15 minutes or
/// any duration from settings (`bot.commands.inactivity_timeout`) of inactivity.
#[derive(Clone)]
pub struct CommandStates(Arc<CommandStatesInner>);

struct CommandStatesInner {
    bot: BotRef,
    futures: TaskTracker,
    // Arc is used to delete any item once the stateful command is concluded finished.
    items: Arc<DashMap<Id<InteractionMarker>, Arc<Mutex<CommandStateInfo>>>>,
    timeout: TimeDelta,
}

impl CommandStates {
    #[must_use]
    pub fn new(bot: BotRef, settings: &Settings) -> Self {
        Self(Arc::new(CommandStatesInner {
            bot,
            futures: TaskTracker::new(),
            items: Arc::new(DashMap::new()),
            timeout: settings.bot.commands.inactivity_timeout,
        }))
    }

    // We already logged interaction id with `interaction.id`
    #[tracing::instrument(skip(self, id))]
    pub fn insert(&self, id: Id<InteractionMarker>, data: StatefulCommand) {
        debug!("creating ephemeral state for interaction {id}");

        let info = Arc::new(Mutex::new(CommandStateInfo {
            data,
            last_used_at: Utc::now(),
        }));
        self.0.items.insert(id, info);
    }

    /// Clears out any inactive stateful commands as long as they reached
    /// the minimum timeout threshold (found in `bot.commands.inactivity_timeout`).
    #[tracing::instrument(skip_all)]
    pub async fn clear_inactive(&self) {
        trace!("clearing all inactive stateful command interactions");

        let mut deletes = Vec::new();
        let now = Utc::now();

        for entry in self.0.items.iter() {
            let id = *entry.key();
            let value = entry.value();

            let command = value.lock().await;
            let difference = (now - command.last_used_at).abs();
            if difference < self.0.timeout {
                continue;
            }

            let bot = self.0.bot.get();
            if let Err(error) = command.data.on_timed_out(&bot).await {
                warn!(%error, "could not process `on_timed_out` for stateful command interaction {id}");
            }
            deletes.push(id);
        }

        let deleted = deletes.len();
        for id in deletes {
            self.0.items.remove(&id);
        }

        if deleted > 0 {
            debug!("cleared {deleted} inactive stateful command interactions");
        } else {
            trace!("cleared 0 inactive stateful command interaction");
        }
    }

    /// Triggers all current stateful commands.
    ///
    /// Not all stateful commands will run as they have set of criteria on
    /// when a stateful command shall continue depending on the given trigger.
    #[tracing::instrument(skip(self))]
    pub fn trigger_commands(&self, trigger: StatefulCommandTrigger) {
        trace!(
            ?trigger,
            "triggering {} stateful command interactions",
            self.0.items.len()
        );

        for entry in self.0.items.iter() {
            let id = *entry.key();
            let value = entry.value().clone();
            let this = self.clone();
            tokio::spawn(async move {
                this.trigger_command(id, value, trigger).await;
            });
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn shutdown(&self) {
        self.0.futures.close();

        let futures = self.0.futures.len();
        if futures == 0 {
            return;
        }

        warn!("waiting for {futures} task(s) to process stateful command interactions");
        self.0.futures.wait().await;
    }

    #[tracing::instrument(skip_all, fields(
        command.data = tracing::field::Empty,
        command.interaction.id = %id,
        command.last_used_at = tracing::field::Empty,
        ?trigger,
    ))]
    async fn trigger_command(
        &self,
        id: Id<InteractionMarker>,
        command: Arc<Mutex<CommandStateInfo>>,
        trigger: StatefulCommandTrigger,
    ) {
        let mut state = command.lock().await;
        let span = Span::current();
        if !span.is_disabled() {
            span.record("command.data", tracing::field::display(&state.data));
            span.record(
                "command.last_used_at",
                tracing::field::debug(&state.last_used_at),
            );
        }

        let bot = self.0.bot.get();
        let action = match state.data.on_trigger(&bot, trigger).await {
            Ok(action) => action,
            Err(error) => {
                warn!(%error, "could not process `on_trigger` for stateful command interaction {id}");
                return;
            }
        };

        trace!("received action = {action:?}");
        match action {
            CommandTriggerAction::Nothing => {}
            CommandTriggerAction::Done => {
                trace!("deleting command state for interaction {id}");
                self.0.items.remove(&id);
            }
            CommandTriggerAction::Continue => {
                state.last_used_at = Utc::now();
            }
        }
    }
}

/// Represents different kinds of stateful commands.
#[derive(Debug, Display)]
pub enum StatefulCommand {
    #[strum(serialize = "PayerApplicationPending")]
    PayerApplicationPending(commands::PayerApplicationPendingState),
    #[strum(serialize = "PayerPayBill")]
    PayerPayBill(commands::PayerPayBillState),
}

/// What [`CommandStates`] should do after the stateful command done
/// every time when there is a [command trigger](StatefulCommandTrigger).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandTriggerAction {
    Done,
    Continue,
    Nothing,
}

/// This type shows the possible triggers that a stateful command may
/// be continue depending on their condition and its implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatefulCommandTrigger {
    /// A user or bot reacted with left arrow emoji.
    ///
    /// It can be used to move into previous page of entries.
    ReactedLeftArrow(Id<MessageMarker>),

    /// A user or bot reacted with right arrow emoji.
    ///
    /// It can be used to move into next page of entries.
    ReactedRightArrow(Id<MessageMarker>),

    /// A user sent a message
    SentMessage(Id<ChannelMarker>, Id<MessageMarker>),
}

struct CommandStateInfo {
    data: StatefulCommand,
    last_used_at: DateTime<Utc>,
}

impl Debug for CommandStates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandStates")
            .field("current_interactions", &self.0.items.len())
            .field("timeout", &self.0.timeout)
            .finish()
    }
}

impl StatefulCommand {
    // We already logged trigger from `CommandStates::trigger_command`
    #[tracing::instrument(skip_all)]
    async fn on_trigger(
        &self,
        bot: &Bot,
        trigger: StatefulCommandTrigger,
    ) -> Result<CommandTriggerAction> {
        match self {
            Self::PayerApplicationPending(data) => data.on_trigger(bot, trigger).await,
            Self::PayerPayBill(data) => data.on_trigger(bot, trigger).await,
        }
    }

    #[tracing::instrument(skip_all)]
    async fn on_timed_out(&self, bot: &Bot) -> Result<()> {
        match self {
            Self::PayerApplicationPending(data) => data.on_timed_out(bot).await,
            Self::PayerPayBill(data) => data.on_timed_out(bot).await,
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait AnyStatefulCommand {
    async fn on_trigger(
        &self,
        bot: &Bot,
        trigger: StatefulCommandTrigger,
    ) -> Result<CommandTriggerAction>;

    async fn on_timed_out(&self, _bot: &Bot) -> Result<()> {
        Ok(())
    }
}
