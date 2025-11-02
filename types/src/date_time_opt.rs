#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
use chrono::{DateTime, SecondsFormat, Utc};
use near_sdk::serde::{self, Deserialize, Deserializer, Serializer};

/// Serialize a timestamp in nanoseconds to date time in ISO 8601 format.
/// E.g. `2025-07-16T16:33:19.000000000Z`.
pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(value) = value {
        if *value > i64::MAX as u64 {
            return Err(serde::ser::Error::custom("Timestamp too large"));
        }

        let date_time = chrono::DateTime::<Utc>::from_timestamp_nanos(*value as i64);
        serializer.serialize_str(&date_time.to_rfc3339_opts(SecondsFormat::Nanos, true))
    } else {
        serializer.serialize_none()
    }
}

/// Deserialize a date time in ISO 8601 format to timestamp in nanoseconds.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer).and_then(|dt: Option<DateTime<Utc>>| {
        dt.map(|dt| {
            dt.timestamp_nanos_opt()
                .map(|nanos| nanos as u64)
                .ok_or_else(|| serde::de::Error::custom("DateTime is out of range"))
        })
        .transpose()
    })
}
