use thiserror::Error;

#[derive(Debug, Error)]
#[error("Eden bot failed")]
pub struct StartBotError;

#[derive(Debug, Error)]
#[error("could not perform database migrations")]
pub struct MigrateError;

#[derive(Debug, Error)]
#[error("could not update local guild admins")]
pub struct UpdateLocalGuildAdminsError;

#[derive(Debug, Error)]
#[error("could not initialize local guild")]
pub struct SetupLocalGuildError;

#[derive(Debug, Error)]
#[error("failed to send welcome message to local guild")]
pub struct SendWelcomeMessageError;

#[derive(Debug, Error)]
#[error("failed to perform HTTP request to Discord")]
pub struct RequestHttpError;

#[derive(Debug, Error)]
#[error("could not register commands")]
pub struct RegisterCommandsError;

pub mod tags {
    use eden_utils::Error;
    use serde::{ser::SerializeMap, Serialize};

    pub fn install_hook() {
        RequestHttpTag::install_hook();
        crate::interactions::tags::install_hook();
    }

    pub struct RequestHttpTag {
        method: twilight_http::request::Method,
        path: String,
    }

    impl Serialize for RequestHttpTag {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            // this is to differentiate various attachments
            let mut map = serializer.serialize_map(Some(3))?;
            map.serialize_entry("_type", "DISCORD_REST")?;
            map.serialize_entry("method", &self.method.to_http().to_string())?;
            map.serialize_entry("path", &self.path)?;
            map.end()
        }
    }

    impl RequestHttpTag {
        pub(crate) fn new(method: twilight_http::request::Method, path: &str) -> Self {
            Self {
                method,
                path: path.into(),
            }
        }

        fn install_hook() {
            Error::install_serde_hook::<Self>();
            Error::install_hook::<Self>(|this, ctx| {
                ctx.push_body(format!("method: {}", this.method.to_http()));
                ctx.push_body(format!("path: {}", this.path));
            });
        }
    }
}
