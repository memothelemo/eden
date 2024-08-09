use doku::Document;
use eden_utils::{error::exts::ResultExt, types::Sensitive, Error, ErrorCategory, Result};
use sentry::types::Dsn;
use serde::{Deserialize, Serialize};

use crate::SettingsLoadError;

#[derive(Debug, Document, Deserialize, Serialize)]
pub struct Sentry {
    #[doku(
        as = "String",
        example = "https://examplePublicKey@o0.ingest.sentry.io/0"
    )]
    pub dsn: Sensitive<Dsn>,
    #[doku(as = "String", example = "release")]
    #[serde(alias = "env")]
    #[serde(default = "Sentry::default_environment")]
    pub environment: String,
    #[doku(example = "1")]
    #[serde(default = "Sentry::default_traces_sample_rate")]
    pub traces_sample_rate: f32,
}

impl Sentry {
    fn default_environment() -> String {
        String::from(eden_utils::build::PROFILE)
    }

    fn default_traces_sample_rate() -> f32 {
        1.
    }

    pub(crate) fn check(&self) -> Result<(), SettingsLoadError> {
        let within_range = self.traces_sample_rate >= 0. && self.traces_sample_rate <= 1.;
        if !within_range {
            return Err(Error::context(ErrorCategory::Unknown, SettingsLoadError))
                .attach_printable("`sentry.traces_sample_rate` must be within range of 0 to 1");
        }

        if self.environment.is_empty() {
            return Err(Error::context(ErrorCategory::Unknown, SettingsLoadError))
                .attach_printable("`sentry.environment` must not be empty");
        }

        Ok(())
    }
}
