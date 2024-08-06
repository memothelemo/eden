use doku::Document;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

#[derive(Debug, Document, Deserialize, Serialize, TypedBuilder)]
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
    #[builder(default = LoggingStyle::Compact)]
    #[doku(as = "String", example = "compact")]
    pub style: LoggingStyle,

    /// This property filters spans and events based on the
    /// set of directives.
    ///
    /// This value may be overriden with `RUST_LOG` if `RUST_LOG` is set
    /// and Eden is built in development mode.
    ///
    /// You may refer on how directives work and parse by going to:
    /// https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/filter/struct.EnvFilter.html
    ///
    /// The default value is a blank string, if not set.
    ///
    /// The default value will filter only events and spans that
    /// have `info` level.
    #[builder(default = "info".into())]
    #[doku(example = "info")]
    pub targets: String,
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
