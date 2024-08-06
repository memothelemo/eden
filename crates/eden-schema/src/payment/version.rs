use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaymentDataVersion(());

impl PaymentDataVersion {
    pub const CURRENT_VERSION: u64 = 1;
}

impl Display for PaymentDataVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&Self::CURRENT_VERSION, f)
    }
}

impl Debug for PaymentDataVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PaymentDataVersion({})", Self::CURRENT_VERSION)
    }
}

impl<'de> Deserialize<'de> for PaymentDataVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        if value != 1 {
            return Err(serde::de::Error::custom(format!(
                "payment data is far ahead from the current version ({value} >= {})",
                PaymentDataVersion::CURRENT_VERSION
            )));
        }

        Ok(Self(()))
    }
}

impl Serialize for PaymentDataVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Self::CURRENT_VERSION.serialize(serializer)
    }
}
