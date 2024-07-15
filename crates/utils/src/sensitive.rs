use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::{Debug, Display};

/// Keeps the raw sensitive data in memory but it cannot be
/// accidentally leaked through the console or logs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Sensitive<T>(T);

impl<T> Sensitive<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: std::ops::Deref> Sensitive<T> {
    pub fn as_deref(&self) -> Sensitive<&T::Target> {
        Sensitive(&*self.0)
    }
}

impl<T: std::ops::Deref> Sensitive<Option<T>> {
    pub fn as_opt_deref(&self) -> Sensitive<Option<&T::Target>> {
        Sensitive(self.0.as_deref())
    }
}

impl Sensitive<String> {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T> Debug for Sensitive<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("<redacted>").finish()
    }
}

impl<T> Display for Sensitive<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("<redacted>").finish()
    }
}

impl<T> AsRef<T> for Sensitive<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl AsRef<str> for Sensitive<String> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<[u8]> for Sensitive<String> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsRef<[u8]> for Sensitive<Vec<u8>> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<T> AsMut<T> for Sensitive<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl AsMut<str> for Sensitive<String> {
    fn as_mut(&mut self) -> &mut str {
        &mut self.0
    }
}

impl std::ops::Deref for Sensitive<String> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Sensitive<String> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for Sensitive<T> {
    fn from(t: T) -> Self {
        Sensitive(t)
    }
}

impl From<&str> for Sensitive<String> {
    fn from(s: &str) -> Self {
        Sensitive(s.into())
    }
}

impl<T> std::borrow::Borrow<T> for Sensitive<T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl std::borrow::Borrow<str> for Sensitive<String> {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl<'a> From<Sensitive<String>> for Cow<'a, str> {
    fn from(val: Sensitive<String>) -> Self {
        Cow::Owned(val.0)
    }
}

impl<'a> From<&'a Sensitive<String>> for Cow<'a, str> {
    fn from(val: &'a Sensitive<String>) -> Self {
        Cow::Borrowed(&val.0)
    }
}

impl<'r, DB: sqlx::Database, T: sqlx::Decode<'r, DB>> sqlx::Decode<'r, DB> for Sensitive<T> {
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let inner = T::decode(value)?;
        Ok(Self(inner))
    }
}

impl<'r, DB: sqlx::Database, T: sqlx::Encode<'r, DB>> sqlx::Encode<'r, DB> for Sensitive<T> {
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::database::HasArguments<'r>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        T::encode_by_ref(&self.0, buf)
    }
}
