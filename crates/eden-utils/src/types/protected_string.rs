// This file is inspired from https://gitlab.com/sequoia-pgp/sequoia/-/blob/main/openpgp/src/crypto/mem.rs
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use zeroize::Zeroize;

// TODO: Encrypt the contents of the string
pub struct ProtectedString {
    data: Box<[u8]>,
    hash: u64,
}

impl ProtectedString {
    #[must_use]
    pub fn new<T: AsRef<str>>(value: T) -> Self {
        let value = value.as_ref();
        let hash = get_string_hash(value);

        let mut data = vec![0; value.len()].into_boxed_slice();
        for (from, to) in value.bytes().zip(data.iter_mut()) {
            *to = from;
        }

        Self { data, hash }
    }

    /// Exposes the value of the string
    #[must_use]
    pub fn expose(&self) -> &str {
        // SAFETY: We already know what is the size of the slice and
        //         we assume that data is inside contains valid UTF-8
        std::str::from_utf8(&self.data).expect("unexpected string is invalid UTF-8")
    }
}

impl From<String> for ProtectedString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl<'a> From<&'a str> for ProtectedString {
    fn from(value: &'a str) -> Self {
        Self::new(value)
    }
}

impl Into<String> for ProtectedString {
    fn into(self) -> String {
        self.expose().to_string()
    }
}

impl Clone for ProtectedString {
    fn clone(&self) -> Self {
        let mut data = vec![0; self.data.len()].into_boxed_slice();
        for (from, to) in self.data.iter().zip(data.iter_mut()) {
            *to = *from;
        }
        Self {
            data,
            hash: self.hash,
        }
    }
}

impl Debug for ProtectedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl Display for ProtectedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl Drop for ProtectedString {
    fn drop(&mut self) {
        let data = self.data.as_mut();
        data.zeroize();
    }
}

// this is to make it unique from u64
#[derive(Hash)]
struct Tag;

impl Hash for ProtectedString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Tag.hash(state);
        self.hash.hash(state);
    }
}

impl PartialEq for ProtectedString {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for ProtectedString {}

impl<T: AsRef<str> + Hash> PartialEq<T> for ProtectedString {
    fn eq(&self, other: &T) -> bool {
        let other_hash = get_string_hash(other);
        self.hash == other_hash
    }
}

impl<'de> Deserialize<'de> for ProtectedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::new(value))
    }
}

impl Serialize for ProtectedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&self.expose())
    }
}

#[must_use]
fn get_string_hash<T: AsRef<str> + Hash>(value: T) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::ProtectedString;
    use static_assertions::assert_impl_all;
    use std::fmt::{Debug, Display};
    use std::hash::Hash;

    assert_impl_all!(
        ProtectedString: Debug,
        Display,
        Clone,
        PartialEq,
        Eq,
        Hash,
        Send,
        Sync
    );

    #[test]
    fn should_generate_same_hash_if_same_content() {
        let a = ProtectedString::new("123");
        let b = ProtectedString::new("123");
        assert_eq!(a.hash, b.hash);
    }

    #[test]
    fn should_have_own_ptr_when_cloned() {
        let a = ProtectedString::new("123");
        let b = a.clone();
        assert_ne!(a.data.as_ptr(), b.data.as_ptr());
    }

    #[test]
    fn should_display_redacted_in_fmts() {
        let value = ProtectedString::new("123");
        assert_eq!(format!("{value:?}"), "<redacted>");
        assert_eq!(format!("{value}"), "<redacted>");
    }

    #[test]
    fn test_partial_eq() {
        let value = ProtectedString::new("123");
        assert_ne!(value, "abc");
        assert_ne!(value, "1234");
        assert_eq!(value, "123");
        assert_eq!(value, value);

        let value2 = ProtectedString::new("123");
        assert_eq!(value, value2);
    }
}
