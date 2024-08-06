use doku::Document;
use eden_utils::types::{ProtectedString, Sensitive};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt::Debug;
use std::num::NonZeroU64;
use std::time::Duration;
use twilight_model::id::marker::GuildMarker;
use twilight_model::id::Id;

#[derive(Debug, Deserialize, Document, Serialize)]
pub struct Bot {
    /// Parameters for configuring what Eden should behave when
    /// it interacts with Discord's REST/HTTP API.
    ///
    /// **Do not modify if you don't know anything about HTTP or how Discord HTTP API works.**
    #[serde(default)]
    http: Http,

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
    local_guild: LocalGuild,

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
    #[doku(example = "")]
    #[serde(default)]
    sharding: Sharding,

    /// This token used to connect and interact with the Discord API.
    ///
    /// **DO NOT SHARE THIS TOKEN TO ANYONE!**
    ///
    /// Your token served as your password to let Discord know that your
    /// bot is trying to interact with Discord. Exposing your Discord bot
    /// token to the public can get access to your bot possibly ruin
    /// anyone's server/guild!
    #[doku(as = "String", example = "<insert token here>")]
    token: ProtectedString,
}

impl Bot {
    #[must_use]
    pub fn http(&self) -> &Http {
        &self.http
    }

    #[must_use]
    pub fn local_guild(&self) -> &LocalGuild {
        &self.local_guild
    }

    #[must_use]
    pub fn sharding(&self) -> &Sharding {
        &self.sharding
    }

    #[must_use]
    pub fn token(&self) -> &ProtectedString {
        &self.token
    }
}

#[derive(Debug, Deserialize, Document, Serialize)]
pub struct LocalGuild {
    /// Eden's central/local guild/server's ID.
    ///
    /// You can get the ID of your desired guild/server by turning on Developer
    /// Mode on Discord then right click the guild/server and click/tap the `Copy Server ID`.
    /// Replace `<insert me>` text with the ID you copied.
    #[doku(as = "String", example = "<insert me>")]
    id: Id<GuildMarker>,
}

impl LocalGuild {
    #[must_use]
    pub fn id(&self) -> Id<GuildMarker> {
        self.id
    }
}

// TODO: allow Eden to do some shard queueing
#[derive(Debug, Deserialize, Document, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Sharding {
    Single {
        /// Assigned id for a single shard/instance
        id: u64,
        /// Total amount of shards needed/should connect to Discord gateway.
        #[doku(as = "u64", example = "1")]
        total: NonZeroU64,
    },
    Range {
        /// Minimum ID that needs to be connected per instance.
        start: u64,

        /// Maximum ID that needs to be connected per instance.
        #[doku(as = "u64", example = "3")]
        end: NonZeroU64,

        /// Total amount of shards needed/should connect to Discord gateway.
        #[doku(as = "u64", example = "5")]
        total: NonZeroU64,
    },
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
    pub(crate) proxy: Option<Sensitive<String>>,

    /// Whether Eden should use HTTP instead of HTTPS to connect
    /// through the proxy server.
    ///
    /// The default value is true if not set.
    #[doku(as = "bool", example = "true")]
    pub(crate) proxy_use_http: bool,

    /// Timeout for every HTTP requests
    ///
    /// The default value is 10 seconds if not set.
    #[doku(as = "String", example = "30m")]
    #[serde_as(as = "eden_utils::serial::AsHumanDuration")]
    pub(crate) timeout: Duration,

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
    pub(crate) use_cache: bool,
}

impl Http {
    #[must_use]
    pub fn use_cache(&self) -> bool {
        self.use_cache
    }

    #[must_use]
    pub fn proxy(&self) -> Option<&str> {
        self.proxy.as_deref()
    }

    #[must_use]
    pub fn proxy_use_http(&self) -> bool {
        self.proxy_use_http
    }

    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
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
