use crate::error::exts::{AnyErrorExt, ErrorExt, IntoError};
use crate::Error;
use thiserror::Error;
use twilight_http::api_error::ApiError;

use super::tags::DiscordHttpErrorInfo;

#[derive(Debug, Error)]
#[error("could not fetch HTTP request to Discord API")]
pub struct FetchHttpError;

pub trait TwilightHttpErrorExt {
    fn discord_http_error_info(&self) -> Option<&DiscordHttpErrorInfo>;
}

impl<T, C> TwilightHttpErrorExt for crate::Result<T, C> {
    fn discord_http_error_info(&self) -> Option<&DiscordHttpErrorInfo> {
        match self {
            Ok(..) => None,
            Err(error) => error.discord_http_error_info(),
        }
    }
}

impl<T> TwilightHttpErrorExt for crate::Error<T> {
    fn discord_http_error_info(&self) -> Option<&DiscordHttpErrorInfo> {
        self.report.request_ref::<DiscordHttpErrorInfo>().next()
    }
}

impl IntoError for twilight_http::Error {
    type Context = FetchHttpError;

    fn into_eden_error(self) -> Error<Self::Context> {
        use twilight_http::error::ErrorType;

        let mut tag = DiscordHttpErrorInfo::Unknown;
        match self.kind() {
            ErrorType::RequestTimedOut => {
                tag = DiscordHttpErrorInfo::TimedOut;
            }
            ErrorType::Response { error, .. } => match error {
                ApiError::General(n) => {
                    tag = DiscordHttpErrorInfo::Response(n.code);
                }
                ApiError::Ratelimited(..) => {
                    tag = DiscordHttpErrorInfo::Ratelimited;
                }
                _ => {}
            },
            ErrorType::ServiceUnavailable { .. } => {
                tag = DiscordHttpErrorInfo::Outage;
            }
            _ => {}
        };

        Error::unknown(self)
            .change_context(FetchHttpError)
            .attach(tag)
    }
}
