#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
use chrono::{DateTime, SecondsFormat, Utc};
use near_sdk::serde::{Deserialize, Deserializer, Serializer};

/// Serialize a timestamp in nanoseconds to date time in ISO 8601 format.
/// E.g. `2025-07-16T16:33:19.000000000Z`.
pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let date_time = chrono::DateTime::<Utc>::from_timestamp_nanos(*value as i64);
    serializer.serialize_str(&date_time.to_rfc3339_opts(SecondsFormat::Nanos, true))
}

/// Deserialize a date time in ISO 8601 format to timestamp in nanoseconds.
pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    DateTime::<Utc>::deserialize(deserializer)
        .map(|dt| dt.timestamp_nanos_opt().unwrap_or_default() as u64)
}
