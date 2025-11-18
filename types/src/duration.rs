use near_sdk::borsh::schema::{Declaration, Definition};
use near_sdk::borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use near_sdk::schemars::{
    JsonSchema,
    r#gen::SchemaGenerator,
    schema::{InstanceType, Metadata, NumberValidation, Schema, SchemaObject},
};
use near_sdk::serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::io::{Read, Write};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
pub struct Duration(std::time::Duration);

impl JsonSchema for Duration {
    fn schema_name() -> String {
        String::schema_name()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let _ = generator; // Duration has no nested definitions.
        let mut schema = SchemaObject::default();
        schema.instance_type = Some(InstanceType::Integer.into());
        schema.number = Some(Box::new(NumberValidation {
            minimum: Some(0.0),
            ..NumberValidation::default()
        }));
        schema.metadata = Some(Box::new(Metadata {
            description: Some("Duration represented as whole seconds".into()),
            ..Metadata::default()
        }));
        Schema::Object(schema)
    }
}

impl BorshSchema for Duration {
    fn add_definitions_recursively(definitions: &mut BTreeMap<Declaration, Definition>) {
        u64::add_definitions_recursively(definitions);
    }

    fn declaration() -> Declaration {
        <u64 as BorshSchema>::declaration()
    }
}

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
