use doku::Document;
use serde::{Deserialize, Serialize};

#[derive(Debug, Document, Deserialize, Serialize)]
#[serde(default)]
pub struct Logging {
    /// Logging style to display logs in a certain style.
    ///
    /// There are three style to choose:
    /// - `compact` - compacts logs but it is readable enough
    /// - `pretty` - makes the entire logs pretty
    /// - `json` - serializes logs into JSON data
    ///
    /// The default value is `compact`, if not set.
    #[doku(as = "String", example = "compact")]
    pub(crate) style: LoggingStyle,

    /// This property filters spans and events based on the
    /// set of directives.
    ///
    /// This value may be overriden with `RUST_LOG` if `RUST_LOG` is set.
    ///
    /// You may refer on how directives work and parse by going to:
    /// https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/filter/struct.EnvFilter.html
    ///
    /// The default value is a blank string, if not set.
    ///
    /// The default value will filter only events and spans that
    /// have `info` level.
    #[doku(example = "info")]
    pub(crate) targets: String,
}

impl Logging {
    #[must_use]
    pub fn style(&self) -> LoggingStyle {
        self.style
    }

    #[must_use]
    pub fn targets(&self) -> &str {
        &self.targets
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LoggingStyle {
    #[default]
    Compact,
    Pretty,
    JSON,
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            style: LoggingStyle::default(),
            targets: String::new(),
        }
    }
}
