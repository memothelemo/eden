use std::borrow::Cow;
use std::fmt::Display;

#[derive(Debug, PartialEq, Eq)]
pub struct Suggestion(Cow<'static, str>);

impl Suggestion {
    #[must_use]
    pub const fn new(message: &'static str) -> Self {
        Self(Cow::Borrowed(message))
    }

    #[must_use]
    pub fn owned(message: impl Display) -> Self {
        let message = message.to_string();
        Self(Cow::Owned(message))
    }
}

impl Suggestion {
    pub(crate) fn install_hook() {
        crate::Error::install_hook::<Self>(move |this, ctx| {
            ctx.push_body(format!("suggestion: {}", this.0));
        });
    }
}
