use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::Deref;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PHPhoneNumber(String);

impl Display for PHPhoneNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<'de> Deserialize<'de> for PHPhoneNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = PHPhoneNumber;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("philipine phone number")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(|e| serde::de::Error::custom(e))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for PHPhoneNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl PHPhoneNumber {
    const LEN: usize = 11;

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<[u8]> for PHPhoneNumber {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsRef<str> for PHPhoneNumber {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for PHPhoneNumber {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Error)]
#[error("unexpected non-digit at offset {0}")]
pub struct InvalidPHPhoneNumber(usize);

impl InvalidPHPhoneNumber {
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.0
    }
}

impl FromStr for PHPhoneNumber {
    type Err = InvalidPHPhoneNumber;

    #[allow(clippy::unwrap_used)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // If it starts with +63, strip it off anyway
        // example: +639123456789 to 09123456789
        let mut s = s.to_string();
        if s.starts_with("+63") {
            let initial = s.strip_prefix("+63").unwrap();
            s = format!("0{initial}");
        }

        if s.len() != Self::LEN {
            return Err(InvalidPHPhoneNumber(s.len()));
        }

        for (offset, c) in s.chars().enumerate() {
            let offset = offset + 1;
            if !c.is_ascii_digit() {
                return Err(InvalidPHPhoneNumber(offset));
            }
        }

        Ok(Self(s.to_string()))
    }
}

/// Mynt's reference number may subject to changes in the future
/// so we need to be adapt to any length but less than 50 digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MyntRefNumber(String);

impl Display for MyntRefNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<'de> Deserialize<'de> for MyntRefNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = MyntRefNumber;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("mynt reference number")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(|e| serde::de::Error::custom(e))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Serialize for MyntRefNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl MyntRefNumber {
    const MAX_CHARS: usize = 50;

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<[u8]> for MyntRefNumber {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsRef<str> for MyntRefNumber {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for MyntRefNumber {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Error)]
pub enum InvalidMyntRefNumber {
    #[error("unexpected non-digit from offset {0} when parsing Mynt reference number")]
    UnexpectedNonDigit(usize),
    #[error("cannot have too many spaces consecutively in Mynt reference number. at offset {0}")]
    TooManySpaces(usize),
    #[error("cannot have too many digits in Mynt reference number")]
    TooManyCharacters,
}

impl InvalidMyntRefNumber {
    #[must_use]
    pub fn offset(&self) -> Option<usize> {
        match self {
            Self::UnexpectedNonDigit(n) | Self::TooManySpaces(n) => Some(*n),
            Self::TooManyCharacters => None,
        }
    }
}

impl FromStr for MyntRefNumber {
    type Err = InvalidMyntRefNumber;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[derive(Debug)]
        enum LastCharacter {
            Space,
            Digit,
            None,
        }

        if s.len() > Self::MAX_CHARS {
            return Err(InvalidMyntRefNumber::TooManyCharacters);
        }

        let mut last_character = LastCharacter::None;
        for (offset, c) in s.chars().enumerate() {
            let offset = offset + 1;

            // You cannot have more than 2 spaces in reference numbers
            if c.is_ascii_digit() {
                last_character = LastCharacter::Digit;
            } else if c == ' ' && matches!(last_character, LastCharacter::Space) {
                return Err(InvalidMyntRefNumber::TooManySpaces(offset));
            } else if c == ' ' {
                last_character = LastCharacter::Space;
            } else {
                return Err(InvalidMyntRefNumber::UnexpectedNonDigit(offset));
            }
        }

        let finalized = s.chars().filter(char::is_ascii_digit).collect::<String>();
        Ok(Self(finalized))
    }
}
