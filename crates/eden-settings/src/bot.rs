use doku::Document;
use eden_utils::error::exts::ErrorExt;
use eden_utils::types::{ProtectedString, Sensitive};
use eden_utils::{Error, ErrorCategory, Result};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use std::fmt::Debug;
use std::num::NonZeroU64;
use std::time::Duration;
use twilight_model::gateway::payload::outgoing::update_presence::UpdatePresencePayload;
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_model::id::Id;
use typed_builder::TypedBuilder;

use crate::SettingsLoadError;

#[derive(Debug, Deserialize, Document, Serialize, TypedBuilder)]
pub struct Bot {
    /// Parameters for configuring what Eden should behave when
    /// it interacts with Discord's REST/HTTP API.
    ///
    /// **Do not modify if you don't know anything about HTTP or how Discord HTTP API works.**
    #[builder(default)]
    #[serde(default)]
    pub http: Http,

    /// "Local guild/server" is where most of Eden's functionality so forth take place
    /// such as payment processes, administration, form applications and many more
    /// to add in the future.
    ///
    /// In the case of Eden project, the bot's local guild/server is Dystopia
    /// (a Discord server).
    ///
    /// You can set up the local guild functionality by pasting your desired
    /// guild/server's ID into the `local_guild.id`/`local_server.id` value.
    ///
    /// This field is not optional as Eden needs a central guild/server to take
    /// advantage of full capabilties of Eden.
    #[serde(alias = "local_server")]
    pub local_guild: LocalGuild,

    /// The default presence of the bot.
    ///
    /// Please refer to the documentation on how to manually configure
    /// presences at: https://discord.com/developers/docs/topics/gateway-events#update-presence-gateway-presence-update-structure
    ///
    /// If it is not set, it will set into a default presence
    /// where no much activity is set for the bot.
    #[builder(default)]
    #[doku(
        as = "HashMap<String, String>",
        example = "status = \"idle\"\nafk = true\n\n[[bot.presence.activities]]\n# Type 0 means playing\ntype = 0\nname = \"with Ferris\"\n# Use this if the type is 1 only\n# url = \"...\"\n\n# created_at = 0 (use Unix timestamps for this field)\n# And many more..."
    )]
    #[serde(default)]
    pub presence: Option<UpdatePresencePayload>,

    /// Parameters for sharding.
    ///
    /// **Do not modify if you don't know anything about sharding in Discord API**
    /// **as carelessly configuring sharding can make:**
    /// - Discord ratelimit you or let your bot token be terminated
    /// - Cloudflare may block you from accessing Discord
    ///
    /// If you want to read about what is sharding, how it works or how should
    /// you configure it, you may visit Discord's developers documentation website at:
    /// https://discord.com/developers/docs/topics/gateway#sharding
    ///
    /// The default configuration of sharding will be a single shard configuration
    /// with an ID of 0 and size of 1 which is sufficient for small bots.
    #[builder(default)]
    #[doku(example = "")]
    #[serde(default)]
    pub sharding: Sharding,

    /// This token used to connect and interact with the Discord API.
    ///
    /// **DO NOT SHARE THIS TOKEN TO ANYONE!**
    ///
    /// Your token served as your password to let Discord know that your
    /// bot is trying to interact with Discord. Exposing your Discord bot
    /// token to the public can get access to your bot possibly ruin
    /// anyone's server/guild!
    #[builder(setter(into))]
    #[doku(as = "String", example = "<insert token here>")]
    pub token: ProtectedString,
}

#[derive(Debug, Deserialize, Document, Serialize, TypedBuilder)]
pub struct LocalGuild {
    /// Eden's central/local guild/server's ID.
    ///
    /// You can get the ID of your desired guild/server by turning on Developer
    /// Mode on Discord then right click the guild/server and click/tap the `Copy Server ID`.
    /// Replace `<insert me>` text with the ID you copied.
    #[doku(as = "String", example = "<insert me>")]
    pub id: Id<GuildMarker>,

    // TODO: Document this field
    /// Alert admin channel.
    #[doku(as = "String", example = "<insert me>")]
    pub alert_channel_id: Id<ChannelMarker>,
}

// TODO: allow Eden to do some shard queueing
#[derive(Deserialize, Document, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Sharding {
    Single {
        /// Assigned id for a single shard/instance
        #[doku(as = "u64", example = "0")]
        id: u64,
        /// Total amount of shards needed to be utilized for the bot.
        #[doku(as = "u64", example = "1")]
        total: NonZeroU64,
    },
    Range {
        /// Minimum ID that needs to be connected per instance.
        start: u64,

        /// Maximum ID that needs to be connected per instance.
        #[doku(as = "u64", example = "3")]
        end: u64,

        /// Total amount of shards needed to be utilized for the bot.
        #[doku(as = "u64", example = "5")]
        total: NonZeroU64,
    },
}

impl Sharding {
    pub const ONE: Self = Sharding::Single {
        id: 0,
        // SAFETY: 1 > 0
        total: unsafe { NonZeroU64::new_unchecked(1) },
    };

    /// First shard index to initialize.
    #[must_use]
    pub fn first(&self) -> u64 {
        match self {
            Self::Single { id, .. } => *id,
            Self::Range { start, .. } => *start,
        }
    }

    /// Number of shards to initialize.
    #[must_use]
    pub fn size(&self) -> u64 {
        match self {
            Self::Single { .. } => 1,
            Self::Range { start, end, .. } => end - start + 1,
        }
    }

    /// Total shards needed to be utilized for the bot.
    #[must_use]
    pub fn total(&self) -> u64 {
        match self {
            Self::Single { total, .. } => total.get(),
            Self::Range { total, .. } => total.get(),
        }
    }
}

impl Sharding {
    // Check the entire configuration if it is configured as intended.
    pub fn check(&self) -> Result<(), SettingsLoadError> {
        match self {
            Self::Single { id, total } => {
                if *id >= total.get() {
                    return Err(Error::context(ErrorCategory::Unknown, SettingsLoadError)
                        .attach_printable(
                            "`sharding.id` should not be equal or greater than the total",
                        ));
                }
            }
            Self::Range { start, end, total } => {
                // start = end is okay but single is recommended
                let start = *start;
                let end = *end;
                let total = total.get();

                if start > end {
                    return Err(Error::context(ErrorCategory::Unknown, SettingsLoadError)
                        .attach_printable(
                            "`sharding.start` should not be more than `sharding.end`",
                        ));
                }

                // start or end must not exceed with the total field
                if start >= total {
                    return Err(Error::context(ErrorCategory::Unknown, SettingsLoadError)
                        .attach_printable(
                            "`sharding.start` should not be equal or more than `sharding.total`",
                        ));
                }

                if end >= total {
                    return Err(Error::context(ErrorCategory::Unknown, SettingsLoadError)
                        .attach_printable(
                            "`sharding.end` should not be equal or more than `sharding.total`",
                        ));
                }
            }
        };
        Ok(())
    }
}

impl Debug for Sharding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Range { start, end, total } => f
                .debug_struct("Range")
                .field("start", start)
                .field("end", end)
                .field("total", &total.get())
                .finish(),
            Self::Single { id, total } => write!(f, "Single([{id}, {}])", total.get()),
        }
    }
}

impl Default for Sharding {
    fn default() -> Self {
        Self::Single {
            id: 0,
            total: NonZeroU64::new(1).unwrap(),
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize, Document, Serialize)]
#[serde(default)]
pub struct Http {
    /// Proxy server to use for all HTTP(S) requests.
    #[doku(as = "String", example = "localhost:1234")]
    pub proxy: Option<Sensitive<String>>,

    /// Whether Eden should use HTTP instead of HTTPS to connect
    /// through the proxy server.
    ///
    /// The default value is true if not set.
    #[doku(as = "bool", example = "true")]
    pub proxy_use_http: bool,

    /// Timeout for every HTTP requests
    ///
    /// The default value is 10 seconds if not set.
    #[doku(as = "String", example = "30m")]
    #[serde_as(as = "eden_utils::serial::AsHumanDuration")]
    pub timeout: Duration,

    /// Using cache allows Eden to minimize amount of REST/HTTP API requests,
    /// requesting too much will lead to ratelimits.
    ///
    /// You may use cache if you don't care about the RAM usage of your
    /// bot, somewhat likely to have outdated data and minimizing the amount
    /// of REST/HTTP API as much as possible, you can enable caching.
    ///
    /// If you want to run Eden with lowest RAM usage as possible,
    /// you may not want to use caching.
    ///
    /// The default value is false if not set.
    #[doku(example = "false")]
    pub use_cache: bool,
}

impl Default for Http {
    fn default() -> Self {
        Self {
            use_cache: false,
            proxy: None,
            proxy_use_http: true,
            timeout: Duration::from_secs(10),
        }
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shard_check() {
        let case = Sharding::ONE;
        assert!(case.check().is_ok());

        let case = Sharding::Single {
            id: 0,
            total: NonZeroU64::new(5).unwrap(),
        };
        assert!(case.check().is_ok());

        let case = Sharding::Single {
            id: 1,
            total: NonZeroU64::new(1).unwrap(),
        };
        assert!(case.check().is_err());

        let case = Sharding::Single {
            id: 2,
            total: NonZeroU64::new(1).unwrap(),
        };
        assert!(case.check().is_err());

        ///////////////////////////////////////////////////
        let case = Sharding::Range {
            start: 0,
            end: 1,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert!(case.check().is_ok());

        let case = Sharding::Range {
            start: 1,
            end: 1,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert!(case.check().is_ok());

        let case = Sharding::Range {
            start: 0,
            end: 0,
            total: NonZeroU64::new(1).unwrap(),
        };
        assert!(case.check().is_ok());

        let case = Sharding::Range {
            start: 0,
            end: 2,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert!(case.check().is_err());

        let case = Sharding::Range {
            start: 2,
            end: 2,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert!(case.check().is_err());

        let case = Sharding::Range {
            start: 2,
            end: 0,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert!(case.check().is_err());

        let case = Sharding::Range {
            start: 2,
            end: 1,
            total: NonZeroU64::new(3).unwrap(),
        };
        assert!(case.check().is_err());
    }

    #[test]
    fn shard_test_first() {
        let default = Sharding::ONE;
        assert_eq!(default.first(), 0);

        let one = Sharding::Single {
            id: 0,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert_eq!(one.first(), 0);

        let one = Sharding::Single {
            id: 1,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert_eq!(one.first(), 1);

        let multiple = Sharding::Range {
            start: 0,
            end: 0,
            total: NonZeroU64::new(1).unwrap(),
        };
        assert_eq!(multiple.first(), 0);

        let multiple = Sharding::Range {
            start: 1,
            end: 3,
            total: NonZeroU64::new(4).unwrap(),
        };
        assert_eq!(multiple.first(), 1);
    }

    #[test]
    fn shard_test_size() {
        let default = Sharding::ONE;
        assert_eq!(default.size(), 1);

        let one = Sharding::Single {
            id: 0,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert_eq!(one.size(), 1);

        let one = Sharding::Single {
            id: 1,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert_eq!(one.size(), 1);

        let multiple = Sharding::Range {
            start: 0,
            end: 0,
            total: NonZeroU64::new(1).unwrap(),
        };
        assert_eq!(multiple.size(), 1);

        let multiple = Sharding::Range {
            start: 1,
            end: 3,
            total: NonZeroU64::new(4).unwrap(),
        };
        assert_eq!(multiple.size(), 3);
    }

    #[test]
    fn shard_test_total() {
        let default = Sharding::ONE;
        assert_eq!(default.total(), 1);

        let one = Sharding::Single {
            id: 0,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert_eq!(one.total(), 2);

        let one = Sharding::Single {
            id: 1,
            total: NonZeroU64::new(2).unwrap(),
        };
        assert_eq!(one.total(), 2);

        let multiple = Sharding::Range {
            start: 0,
            end: 0,
            total: NonZeroU64::new(1).unwrap(),
        };
        assert_eq!(multiple.total(), 1);

        let multiple = Sharding::Range {
            start: 1,
            end: 3,
            total: NonZeroU64::new(4).unwrap(),
        };
        assert_eq!(multiple.total(), 4);
    }
}
