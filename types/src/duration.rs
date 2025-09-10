use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io::{Read, Write};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct Duration(std::time::Duration);

impl Duration {
    #[must_use]
    pub const fn from_secs(seconds: u64) -> Self {
        Self(std::time::Duration::from_secs(seconds))
    }

    #[must_use]
    pub const fn from_nanos(nanoseconds: u64) -> Self {
        Self(std::time::Duration::from_nanos(nanoseconds))
    }

    #[must_use]
    pub const fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }

    #[must_use]
    pub fn as_nanos(&self) -> u64 {
        self.0.as_nanos().try_into().unwrap_or(u64::MAX)
    }
}

impl From<u64> for Duration {
    fn from(nanoseconds: u64) -> Self {
        Self::from_nanos(nanoseconds)
    }
}

impl BorshSerialize for Duration {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(&self.as_secs(), writer)
    }
}

impl BorshDeserialize for Duration {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let seconds = u64::deserialize_reader(reader)?;
        Ok(Self::from_secs(seconds))
    }
}

impl Serialize for Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.as_secs())
    }
}

impl<'de> Deserialize<'de> for Duration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(Self::from_secs)
    }
}
