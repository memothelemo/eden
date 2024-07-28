use doku::Document;
use eden_utils::Sensitive;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt::Debug;
use std::time::Duration;
use twilight_model::id::marker::ApplicationMarker;
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

    /// Parameters for configuring what Eden should behave when
    /// it interacts with Discord's REST/HTTP API.
    ///
    /// **Do not modify if you don't know anything about HTTP or how Discord HTTP API works.**
    #[serde(default)]
    pub(crate) http: Http,
}

impl Bot {
    #[must_use]
    pub fn http(&self) -> &Http {
        &self.http
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
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
    /// The default is true if not set.
    #[doku(as = "bool", example = "true")]
    pub(crate) proxy_use_http: bool,

    /// Timeout for every HTTP requests
    ///
    /// The default is 10 seconds if not set.
    #[doku(as = "String", example = "30m")]
    #[serde_as(as = "eden_utils::serial::AsHumanDuration")]
    pub(crate) timeout: Duration,
}

impl Http {
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
            proxy: None,
            proxy_use_http: true,
            timeout: Duration::from_secs(10),
        }
    }
}
