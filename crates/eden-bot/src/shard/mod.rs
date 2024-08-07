// This sharding architecture is inspired from serenity.
mod manager;
mod observer;
mod runner;

pub use self::manager::ShardManager;
pub use self::runner::ShardHandle;

use chrono::{DateTime, Utc};
use std::fmt::Debug;
use twilight_model::gateway::{
    payload::outgoing::{update_presence::UpdatePresencePayload, UpdatePresence},
    presence::{Activity, Status},
};

#[derive(Debug)]
#[must_use]
pub struct PresenceData {
    pub activites: Vec<Activity>,
    pub afk: bool,
    pub since: Option<DateTime<Utc>>,
    pub status: Status,
}

impl PresenceData {
    /// Creates a default [`PresenceData`].
    pub fn new() -> Self {
        Self::default()
    }

    pub fn activity(mut self, activity: Activity) -> Self {
        self.activites.push(activity);
        self
    }

    pub fn since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    pub fn status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// Trnsforms from [`PresenceData`] into [`UpdatePresence`].
    fn transform(&self) -> UpdatePresence {
        UpdatePresence {
            d: UpdatePresencePayload {
                activities: self.activites.clone(),
                afk: self.afk,
                // https://discord.com/developers/docs/topics/gateway-events#update-presence-gateway-presence-update-structure
                since: match self.status {
                    Status::Idle => self.since.map(|v| v.timestamp_millis() as u64),
                    _ => None,
                },
                status: self.status,
            },
            op: twilight_model::gateway::OpCode::PresenceUpdate,
        }
    }
}

impl Default for PresenceData {
    fn default() -> Self {
        Self {
            activites: Vec::new(),
            afk: false,
            since: None,
            status: Status::Online,
        }
    }
}
