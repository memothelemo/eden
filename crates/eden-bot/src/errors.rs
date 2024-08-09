use thiserror::Error;

#[derive(Debug, Error)]
#[error("Eden bot failed")]
pub struct StartBotError;

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

    pub fn install_hook() {
        RequestHttpTag::install_hook();
        crate::interactions::tags::install_hook();
    }

    pub struct RequestHttpTag {
        method: twilight_http::request::Method,
        path: String,
    }

    impl RequestHttpTag {
        pub(crate) fn new(method: twilight_http::request::Method, path: &str) -> Self {
            Self {
                method,
                path: path.into(),
            }
        }

        fn install_hook() {
            Error::install_hook::<Self>(|this, ctx| {
                ctx.push_body(format!("method: {}", this.method.to_http()));
                ctx.push_body(format!("path: {}", this.path));
            });
        }
    }
}
