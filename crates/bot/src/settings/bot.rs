use doku::Document;
use eden_utils::Sensitive;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt::Debug;
use std::num::NonZeroU64;
use std::time::Duration;
use twilight_model::id::marker::{ApplicationMarker, GuildMarker, UserMarker};
use twilight_model::id::Id;

#[derive(Debug, Deserialize, Document, Serialize)]
pub struct Bot {
    /// Application ID of the bot.
    ///
    /// This configuration is typically not required to set as Eden
    /// will retrieve the application ID of the bot after it successfully
    /// connects to Discord.
    ///
    /// However, you should put your bot's application ID if you're expecting
    /// your Eden instance will frequently crash, restart or throw an error
    /// to avoid getting rate limited from Discord.
    #[doku(as = "String", example = "745809834183753828")]
    pub(crate) application_id: Option<Id<ApplicationMarker>>,

    /// A list of developers that have special privileges to Eden unlike
    /// standard users. Special privileges include:
    ///
    /// - Able to see the entire error of why it isn't working.
    #[doku(as = "Vec<String>", example = "[\"876711213126520882\"]")]
    pub(crate) developers: Vec<Id<UserMarker>>,

    /// Parameters for configuring what Eden should behave when
    /// dealing with commands and operations inside your guild/server.
    #[serde(alias = "server")]
    pub(crate) guild: Guild,

    /// Parameters for configuring what Eden should behave when
    /// it interacts with Discord's REST/HTTP API.
    ///
    /// **Do not modify if you don't know anything about HTTP or how Discord HTTP API works.**
    #[serde(default)]
    pub(crate) http: Http,

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
    #[serde(default)]
    pub(crate) sharding: Sharding,

    // TODO: Find a way on how to securely keep the token from accessing it from the instance's memory in its raw value
    /// This token used to connect and interact with the Discord API.
    ///
    /// **DO NOT SHARE THIS TOKEN TO ANYONE!**
    ///
    /// Your token served as your password to let Discord know that your
    /// bot is trying to interact with Discord. Exposing your Discord bot
    /// token to the public can get access to your bot possibly ruin
    /// anyone's server/guild!
    #[doku(as = "String", example = "<insert token here>")]
    pub(crate) token: Sensitive<String>,
}

impl Bot {
    #[must_use]
    pub fn application_id(&self) -> Option<Id<ApplicationMarker>> {
        self.application_id
    }

    #[must_use]
    pub fn is_developer_user(&self, user_id: Id<UserMarker>) -> bool {
        self.developers.iter().any(|v| *v == user_id)
    }

    #[must_use]
    pub fn guild(&self) -> &Guild {
        &self.guild
    }

    #[must_use]
    pub fn http(&self) -> &Http {
        &self.http
    }

    #[must_use]
    pub fn sharding(&self) -> &Sharding {
        &self.sharding
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Debug, Deserialize, Document, Serialize)]
pub struct Guild {
    /// Whether Eden allows guild administrators (who have ADMINISTRATOR permission)
    /// to register themselves as payers without the approval of other guild
    /// administrators.
    ///
    /// If this option is disabled, the guild administrator must wait for any
    /// guild administrators to approve their registration.
    ///
    /// You should allow self-registration if there's only you or one guild
    /// administrators in your chosen guild. Otherwise, it is recommended to
    /// not to do so.
    ///
    /// The default value if not set is true.
    #[serde(default = "Guild::default_allow_self_payer_registration")]
    pub(crate) allow_self_payer_registration: bool,

    /// Your guild/server's ID.
    ///
    /// It is required as this is most of the time the bot is interacting
    /// with and so with the members of your guild/server.
    #[doku(as = "String", example = "442252698964721669")]
    pub(crate) id: Id<GuildMarker>,
}

impl Guild {
    #[must_use]
    pub fn allow_self_payer_registration(&self) -> bool {
        self.allow_self_payer_registration
    }

    #[must_use]
    pub fn id(&self) -> Id<GuildMarker> {
        self.id
    }
}

impl Guild {
    fn default_allow_self_payer_registration() -> bool {
        true
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
        // /// How many shards can successfully connect per 5 seconds.
        // ///
        // /// If you don't know what to set, it is recommended to set this
        // /// to 1 (default value). It means that every 1 shard will successfully
        // /// connect to Discord at a time per 5 seconds.
        // #[serde(default = "Sharding::default_concurrency")]
        // #[doku(as = "u64", example = "1")]
        // concurrency: NonZeroU64,
    },
}

// impl Sharding {
//     #[allow(clippy::unwrap_used)]
//     fn default_concurrency() -> NonZeroU64 {
//         NonZeroU64::new(1).unwrap()
//     }
// }

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
