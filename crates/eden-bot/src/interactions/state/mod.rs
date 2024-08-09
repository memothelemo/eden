use chrono::{DateTime, TimeDelta, Utc};
use dashmap::DashMap;
use eden_utils::Result;
use std::fmt::{Debug, Display};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, trace, warn};
use twilight_model::id::marker::{ChannelMarker, InteractionMarker, MessageMarker};
use twilight_model::id::Id;

use crate::{Bot, BotRef};

pub mod stateful_commands;

/// Holds state of all commands used in this bot.
///
/// Every interaction state will be wiped after 15 minutes of inactivity.
pub struct InMemoryCommandState {
    bot: BotRef,
    items: Arc<DashMap<MaybeInteractionId, Arc<Mutex<CommandState>>>>,
}

// TODO: Refactor the entire thing! :(
impl InMemoryCommandState {
    // TODO: Make interaction state expiration configurable
    #[must_use]
    pub fn new(bot: BotRef) -> Arc<Self> {
        Arc::new(Self {
            bot,
            items: Arc::new(DashMap::new()),
        })
    }

    #[tracing::instrument(skip(self, id))]
    pub fn insert(&self, id: impl Into<MaybeInteractionId>, data: StatefulCommandType) {
        let id = id.into();
        debug!("inserting state with id {id}");

        self.items
            .insert(id, Arc::new(Mutex::new(CommandState::new(data))));
    }

    #[tracing::instrument(skip(self))]
    pub async fn trigger_command(&self, trigger: StatefulCommandTrigger) {
        trace!("triggering command with {trigger:?}");

        for entry in self.items.iter() {
            let id = *entry.key();
            let entry = entry.clone();

            let bot = self.bot.get();
            let trigger = trigger.clone();
            let items = self.items.clone();

            tokio::spawn(async move {
                let mut state = entry.lock().await;
                match state.kind.on_trigger(&bot, trigger).await {
                    Ok(StatefulCommandResult::Ignore) => return,
                    Ok(StatefulCommandResult::Continue) => {
                        state.last_used_at = Utc::now();
                        return;
                    }
                    Ok(StatefulCommandResult::Done) => {}
                    Err(error) => {
                        warn!(%error, "failed to run on_trigger");
                        return;
                    }
                };
                drop(state);

                trace!("deleting state with id {id}");
                items.remove(&id);
            });
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn clear_inactive(&self) {
        debug!("clearing all inactive interactions' state");

        let mut to_delete = Vec::new();
        let now = Utc::now();
        for entry in self.items.iter() {
            let key = *entry.key();
            let entry = entry.clone();
            let state = entry.lock().await;
            let difference = (now - state.last_used_at).abs();
            trace!(interaction.id = %key, elapsed = ?difference);

            if difference >= TimeDelta::minutes(15) {
                let bot = &self.bot.get();
                if let Err(error) = state.kind.on_inactive(bot).await {
                    warn!(%error, "failed to run on_active");
                }
                to_delete.push(key);
            }
        }

        let to_delete_len = to_delete.len();
        for id in to_delete {
            self.items.remove(&id);
        }

        debug!("cleared {to_delete_len} inactive interaction states");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaybeInteractionId {
    Message(Id<MessageMarker>),
    Interaction(Id<InteractionMarker>),
}

impl Display for MaybeInteractionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(n) => Display::fmt(&n, f),
            Self::Interaction(n) => Display::fmt(&n, f),
        }
    }
}

impl From<Id<MessageMarker>> for MaybeInteractionId {
    fn from(value: Id<MessageMarker>) -> Self {
        Self::Message(value)
    }
}

impl From<Id<InteractionMarker>> for MaybeInteractionId {
    fn from(value: Id<InteractionMarker>) -> Self {
        Self::Interaction(value)
    }
}

#[derive(Debug)]
pub struct CommandState {
    pub kind: StatefulCommandType,
    #[allow(unused)]
    pub invoked_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

impl CommandState {
    #[must_use]
    pub fn new(kind: StatefulCommandType) -> Self {
        let now = Utc::now();
        Self {
            kind,
            invoked_at: now,
            last_used_at: now,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StatefulCommandTrigger {
    ReactedLeftArrow(Id<MessageMarker>),
    ReactedRightArrow(Id<MessageMarker>),
    SentMessage(Id<ChannelMarker>, Id<MessageMarker>),
}

#[derive(Debug)]
pub enum StatefulCommandType {
    PayerApplicationPending(stateful_commands::PayerApplicationPending),
    PayerPayBill(stateful_commands::PayerPayBill),
}

impl StatefulCommandType {
    #[tracing::instrument(skip(bot))]
    async fn on_trigger(
        &self,
        bot: &Bot,
        trigger: StatefulCommandTrigger,
    ) -> Result<StatefulCommandResult> {
        match self {
            Self::PayerApplicationPending(n) => n.on_trigger(bot, trigger).await,
            Self::PayerPayBill(n) => n.on_trigger(bot, trigger).await,
        }
    }

    #[tracing::instrument(skip(bot))]
    async fn on_inactive(&self, bot: &Bot) -> Result<()> {
        match self {
            Self::PayerApplicationPending(n) => n.on_inactive(bot).await,
            Self::PayerPayBill(n) => n.on_inactive(bot).await,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatefulCommandResult {
    Continue,
    Done,
    Ignore,
}

#[allow(async_fn_in_trait)]
pub trait StatefulCommand {
    async fn on_trigger(
        &self,
        bot: &Bot,
        trigger: StatefulCommandTrigger,
    ) -> Result<StatefulCommandResult>;
    async fn on_inactive(&self, _bot: &Bot) -> Result<()> {
        Ok(())
    }
}

impl Debug for InMemoryCommandState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryInteractionState")
            .field("interactions", &self.items.len())
            .finish()
    }
}
