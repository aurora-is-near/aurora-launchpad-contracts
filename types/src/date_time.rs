#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
use chrono::{DateTime, SecondsFormat, Utc};
use near_sdk::serde::{self, Deserialize, Deserializer, Serializer};

/// Serialize a timestamp in nanoseconds to date time in ISO 8601 format.
/// E.g. `2025-07-16T16:33:19.000000000Z`.
pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if *value > i64::MAX as u64 {
        return Err(serde::ser::Error::custom("Timestamp too large"));
    }

    let date_time = chrono::DateTime::<Utc>::from_timestamp_nanos(*value as i64);
    serializer.serialize_str(&date_time.to_rfc3339_opts(SecondsFormat::Nanos, true))
}

/// Deserialize a date time in ISO 8601 format to timestamp in nanoseconds.
pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    DateTime::<Utc>::deserialize(deserializer).and_then(|dt| {
        dt.timestamp_nanos_opt()
            .ok_or_else(|| serde::de::Error::custom("DateTime is out of range"))
            .map(|nanos| nanos as u64)
    })
}

/// Custom serde module for (de)serializing seconds and converting them into(from) nanoseconds.
pub mod nanos_to_seconds {
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    /// Convert nanoseconds to seconds and serialize them.
    pub fn serialize<S>(nanoseconds: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(*nanoseconds / 1_000_000_000)
    }

    /// Deserialize seconds and convert them into nanoseconds.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(|seconds| seconds * 1_000_000_000)
    }
}
