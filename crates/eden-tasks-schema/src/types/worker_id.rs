use serde::{de::IgnoredAny, ser::SerializeSeq, Deserialize, Serialize};
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerId {
    /// Assigned number of the worker. It starts from 1 only.
    assigned: NonZeroU32,
    /// Total number of workers used by the system/bot. It starts from 1 only.
    total: NonZeroU32,
}

impl Default for WorkerId {
    fn default() -> Self {
        WorkerId::ONE
    }
}

impl WorkerId {
    pub const ONE: Self = Self::new(1, 1);

    #[must_use]
    pub const fn new(assigned: u32, total: u32) -> Self {
        assert!(
            assigned <= total,
            "assigned must be less than or equal total"
        );

        let Some(assigned) = NonZeroU32::new(assigned) else {
            panic!("assigned must be at least 1");
        };

        let Some(total) = NonZeroU32::new(total) else {
            panic!("total must be at least 1");
        };

        Self { assigned, total }
    }

    pub const fn new_checked(assigned: u32, total: u32) -> Option<Self> {
        let assigned = match NonZeroU32::new(assigned) {
            Some(n) => n,
            None => return None,
        };

        let total = match NonZeroU32::new(total) {
            Some(n) => n,
            None => return None,
        };

        Some(Self { assigned, total })
    }

    #[must_use]
    pub const fn assigned(self) -> u32 {
        self.assigned.get()
    }

    #[must_use]
    pub const fn total(self) -> u32 {
        self.total.get()
    }

    // Not much data loss converting from u32 to i64
    pub(crate) const fn assigned_sql(self) -> i64 {
        self.assigned.get() as i64
    }

    // Not much data loss converting from u32 to i64
    pub(crate) const fn total_sql(self) -> i64 {
        self.total.get() as i64
    }
}

impl std::fmt::Display for WorkerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.assigned(), self.total())
    }
}

impl PartialOrd for WorkerId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.total != other.total {
            None
        } else {
            self.assigned.partial_cmp(&other.assigned)
        }
    }
}

impl From<WorkerId> for (u32, u32) {
    fn from(value: WorkerId) -> Self {
        (value.assigned(), value.total())
    }
}

impl<'de> Deserialize<'de> for WorkerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Assigned,
            Id,
            Total,
            #[serde(other)]
            Ignore,
        }

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = WorkerId;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Eden worker id")
            }

            // [0, 1]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let assigned: NonZeroU32 = seq.next_element()?.ok_or_else(|| {
                    serde::de::Error::invalid_length(0, &"worker id with 2 elements")
                })?;
                let total: NonZeroU32 = seq.next_element()?.ok_or_else(|| {
                    serde::de::Error::invalid_length(0, &"worker id with 2 elements")
                })?;

                if assigned.get() > total.get() {
                    return Err(serde::de::Error::custom(
                        "assigned must be less than or equal total",
                    ));
                }

                Ok(WorkerId { assigned, total })
            }

            // { "assigned": 1, "total": 2 } / { "id": 1, "total": 2 }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut assigned: Option<NonZeroU32> = None;
                let mut id: Option<NonZeroU32> = None;
                let mut total: Option<NonZeroU32> = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Assigned => {
                            if assigned.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            assigned = Some(map.next_value()?);
                        }
                        Field::Id => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        Field::Total => {
                            if total.is_some() {
                                return Err(serde::de::Error::duplicate_field("total"));
                            }
                            total = Some(map.next_value()?);
                        }
                        Field::Ignore => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }

                if assigned.is_some() && id.is_some() {
                    return Err(serde::de::Error::custom(
                        "'assigned' and 'id' cannot be used at the same time",
                    ));
                }

                let assigned = assigned
                    .or(id)
                    .ok_or_else(|| serde::de::Error::missing_field("assigned"))?;

                let total = total.ok_or_else(|| serde::de::Error::missing_field("total"))?;
                if assigned.get() > total.get() {
                    return Err(serde::de::Error::custom(
                        "assigned must be less than or equal total",
                    ));
                }

                Ok(WorkerId { assigned, total })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl Serialize for WorkerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_seq(Some(2))?;
        map.serialize_element(&self.assigned())?;
        map.serialize_element(&self.total())?;
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::WorkerId;
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};
    use static_assertions::{assert_impl_all, const_assert_eq};

    const_assert_eq!(WorkerId::ONE.assigned(), 1);
    const_assert_eq!(WorkerId::ONE.total(), 1);

    assert_impl_all!(
        WorkerId: std::fmt::Debug,
        std::fmt::Display,
        Clone, Copy,
        PartialEq, Eq,
        PartialOrd,
        Send, Sync
    );

    #[test]
    fn test_serde_with_map() {
        let id = serde_json::from_str::<WorkerId>(r#"{"assigned":1,"total":1}"#).unwrap();
        assert_eq!(id, WorkerId::ONE);

        assert_de_tokens_error::<WorkerId>(
            &[
                Token::Map { len: Some(2) },
                Token::Str("assigned"),
                Token::U32(2),
                Token::Str("total"),
                Token::U32(1),
                Token::MapEnd,
            ],
            "assigned must be less than or equal total",
        );
    }

    #[test]
    fn test_serde_with_map_and_alias() {
        let id = serde_json::from_str::<WorkerId>(r#"{"id":1,"total":1}"#).unwrap();
        assert_eq!(id, WorkerId::ONE);

        assert_de_tokens_error::<WorkerId>(
            &[
                Token::Map { len: Some(2) },
                Token::Str("id"),
                Token::U32(2),
                Token::Str("total"),
                Token::U32(1),
                Token::MapEnd,
            ],
            "assigned must be less than or equal total",
        );
    }

    #[test]
    fn test_serde_with_sequence() {
        assert_tokens(
            &WorkerId::ONE,
            &[
                Token::Seq { len: Some(2) },
                Token::U32(1),
                Token::U32(1),
                Token::SeqEnd,
            ],
        );

        assert_de_tokens_error::<WorkerId>(
            &[
                Token::Seq { len: Some(2) },
                Token::U32(2),
                Token::U32(1),
                Token::SeqEnd,
            ],
            "assigned must be less than or equal total",
        );
    }
}
