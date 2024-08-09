// This sharding architecture is inspired from serenity.
mod manager;
mod observer;
mod runner;

pub use self::manager::ShardManager;
pub use self::runner::ShardHandle;
pub use twilight_model::gateway::presence::{
    Activity, ActivityAssets, ActivityButton, ActivityEmoji, ActivityFlags, ActivityParty,
    ActivitySecrets, ActivityTimestamps, ActivityType, Status as PresenceStatus,
};

use chrono::{DateTime, Utc};
use std::fmt::{Debug, Display};
use twilight_model::{
    gateway::payload::outgoing::update_presence::UpdatePresencePayload,
    id::{marker::ApplicationMarker, Id},
};

#[derive(Debug)]
#[must_use]
pub struct ActivityBuilder(Activity);

impl ActivityBuilder {
    pub fn new(kind: ActivityType, name: impl Display) -> Self {
        Self(Activity {
            application_id: None,
            assets: None,
            buttons: Vec::new(),
            created_at: None,
            details: None,
            emoji: None,
            flags: None,
            id: None,
            instance: None,
            kind,
            name: name.to_string(),
            party: None,
            secrets: None,
            state: None,
            timestamps: None,
            url: None,
        })
    }

    pub fn application_id(mut self, value: Id<ApplicationMarker>) -> Self {
        self.0.application_id = Some(value);
        self
    }

    pub fn assets(mut self, value: ActivityAssets) -> Self {
        self.0.assets = Some(value);
        self
    }

    pub fn button(mut self, value: ActivityButton) -> Self {
        self.0.buttons.push(value);
        self
    }

    pub fn created_at(mut self, value: DateTime<Utc>) -> Self {
        self.0.created_at = Some(value.timestamp_millis() as u64);
        self
    }

    pub fn details(mut self, value: String) -> Self {
        self.0.details = Some(value);
        self
    }

    pub fn emoji(mut self, value: ActivityEmoji) -> Self {
        self.0.emoji = Some(value);
        self
    }

    pub fn flags(mut self, value: ActivityFlags) -> Self {
        self.0.flags = Some(value);
        self
    }

    pub fn id(mut self, value: String) -> Self {
        self.0.id = Some(value);
        self
    }

    pub fn instance(mut self, value: bool) -> Self {
        self.0.instance = Some(value);
        self
    }

    pub fn party(mut self, value: ActivityParty) -> Self {
        self.0.party = Some(value);
        self
    }

    pub fn secrets(mut self, value: ActivitySecrets) -> Self {
        self.0.secrets = Some(value);
        self
    }

    pub fn state(mut self, value: String) -> Self {
        self.0.state = Some(value);
        self
    }

    pub fn timestamps(mut self, value: ActivityTimestamps) -> Self {
        self.0.timestamps = Some(value);
        self
    }

    pub fn url(mut self, value: String) -> Self {
        self.0.url = Some(value);
        self
    }

    #[must_use]
    pub fn build(self) -> Activity {
        self.0
    }
}

#[derive(Debug, Clone)]
#[must_use]
pub struct PresenceData {
    pub activities: Vec<Activity>,
    pub afk: bool,
    pub since: Option<DateTime<Utc>>,
    pub status: PresenceStatus,
}

impl From<PresenceData> for UpdatePresencePayload {
    fn from(value: PresenceData) -> Self {
        value.transform_to_payload()
    }
}

impl PresenceData {
    /// Creates a default [`PresenceData`].
    pub fn new() -> Self {
        Self::default()
    }

    pub fn activity(mut self, activity: Activity) -> Self {
        self.activities.push(activity);
        self
    }

    pub fn since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    pub fn status(mut self, status: PresenceStatus) -> Self {
        self.status = status;
        self
    }

    /// Trnsforms from [`PresenceData`] into [`UpdatePresencePayload`].
    fn transform_to_payload(&self) -> UpdatePresencePayload {
        // We're manually creating update presence since twilight
        // won't allow us to use `new` function without getting an error
        // if an empty set of activities is provided.
        UpdatePresencePayload {
            activities: self.activities.clone(),
            afk: self.afk,
            // https://discord.com/developers/docs/topics/gateway-events#update-presence-gateway-presence-update-structure
            since: match self.status {
                PresenceStatus::Idle => self.since.map(|v| v.timestamp_millis() as u64),
                _ => None,
            },
            status: self.status,
        }
    }
}

impl Default for PresenceData {
    fn default() -> Self {
        Self {
            activities: Vec::new(),
            afk: false,
            since: None,
            status: PresenceStatus::Online,
        }
    }
}
