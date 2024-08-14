use serde::ser::SerializeMap;
use serde::Serialize;
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

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl AsRef<str> for Suggestion {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Serialize for Suggestion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("suggestion", &self.0)?;
        map.end()
    }
}

impl Suggestion {
    pub(crate) fn install_hook() {
        crate::Error::install_serde_hook::<Self>();
        crate::Error::install_hook::<Self>(move |this, ctx| {
            ctx.push_body(format!("suggestion: {}", this.0));
        });
    }
}
