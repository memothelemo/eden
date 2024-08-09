use chrono::TimeDelta;
use fundu::DurationParser;
use serde_with::{DeserializeAs, SerializeAs};
use std::time::Duration as StdDuration;

pub struct AsHumanDuration;

struct StdVisitor;

impl<'de> serde::de::Visitor<'de> for StdVisitor {
    type Value = StdDuration;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("human duration")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        use fundu::TimeUnit;
        use serde::de::Error as DeError;

        const PARSER: DurationParser<'static> = DurationParser::builder()
            .time_units(&[
                TimeUnit::MilliSecond,
                TimeUnit::Second,
                TimeUnit::Minute,
                TimeUnit::Hour,
                TimeUnit::Day,
            ])
            .allow_time_unit_delimiter()
            .disable_exponent()
            .build();

        let parsed = PARSER.parse(v).map_err(DeError::custom)?;
        StdDuration::try_from(parsed).map_err(DeError::custom)
    }
}

struct ChronoVisitor;

impl<'de> serde::de::Visitor<'de> for ChronoVisitor {
    type Value = TimeDelta;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("human duration")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        use fundu::TimeUnit;
        use serde::de::Error as DeError;

        const PARSER: DurationParser<'static> = DurationParser::builder()
            .time_units(&[
                TimeUnit::MilliSecond,
                TimeUnit::Second,
                TimeUnit::Minute,
                TimeUnit::Hour,
                TimeUnit::Day,
            ])
            .allow_time_unit_delimiter()
            .disable_exponent()
            .build();

        let parsed = PARSER.parse(v).map_err(DeError::custom)?;
        TimeDelta::try_from(parsed).map_err(DeError::custom)
    }
}

impl<'de> DeserializeAs<'de, StdDuration> for AsHumanDuration {
    fn deserialize_as<D>(deserializer: D) -> Result<StdDuration, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(StdVisitor)
    }
}

impl SerializeAs<StdDuration> for AsHumanDuration {
    fn serialize_as<S>(source: &StdDuration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let duration: fundu::Duration = (*source).into();
        serializer.collect_str(&duration.to_string())
    }
}

impl<'de> DeserializeAs<'de, TimeDelta> for AsHumanDuration {
    fn deserialize_as<D>(deserializer: D) -> Result<TimeDelta, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ChronoVisitor)
    }
}

impl SerializeAs<TimeDelta> for AsHumanDuration {
    fn serialize_as<S>(source: &TimeDelta, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let duration: fundu::Duration = (*source).into();
        serializer.collect_str(&duration.to_string())
    }
}
