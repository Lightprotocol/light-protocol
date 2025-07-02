use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper type that can deserialize from either a u64 or a string
#[derive(Clone, Debug, PartialEq, Default)]
pub struct StringU64(pub u64);

impl StringU64 {
    pub fn new(value: u64) -> Self {
        StringU64(value)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for StringU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for StringU64 {
    fn from(value: u64) -> Self {
        StringU64(value)
    }
}

impl From<StringU64> for u64 {
    fn from(value: StringU64) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for StringU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrU64 {
            String(String),
            U64(u64),
        }

        match StringOrU64::deserialize(deserializer)? {
            StringOrU64::String(s) => s
                .parse::<u64>()
                .map(StringU64)
                .map_err(serde::de::Error::custom),
            StringOrU64::U64(n) => Ok(StringU64(n)),
        }
    }
}

impl Serialize for StringU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Always serialize as string to match what the server expects
        serializer.serialize_str(&self.0.to_string())
    }
}

/// Helper module for optional StringU64 fields
pub mod option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::StringU64;

    pub fn serialize<S>(value: &Option<StringU64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) => v.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<StringU64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<StringU64>::deserialize(deserializer)
    }
}

/// Direct deserialization helpers for u64 fields
pub mod direct {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrU64 {
            String(String),
            U64(u64),
        }

        match StringOrU64::deserialize(deserializer)? {
            StringOrU64::String(s) => s.parse::<u64>().map_err(serde::de::Error::custom),
            StringOrU64::U64(n) => Ok(n),
        }
    }

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }
}

/// Direct deserialization helpers for optional u64 fields
pub mod option_direct {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrU64 {
            String(String),
            U64(u64),
        }

        let opt = Option::<StringOrU64>::deserialize(deserializer)?;
        match opt {
            Some(StringOrU64::String(s)) => {
                s.parse::<u64>().map(Some).map_err(serde::de::Error::custom)
            }
            Some(StringOrU64::U64(n)) => Ok(Some(n)),
            None => Ok(None),
        }
    }

    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) => serializer.serialize_str(&v.to_string()),
            None => serializer.serialize_none(),
        }
    }
}

/// Direct deserialization helpers for u32 fields
pub mod u32_direct {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrU32 {
            String(String),
            U32(u32),
        }

        match StringOrU32::deserialize(deserializer)? {
            StringOrU32::String(s) => s.parse::<u32>().map_err(serde::de::Error::custom),
            StringOrU32::U32(n) => Ok(n),
        }
    }

    pub fn serialize<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }
}

/// Direct deserialization helpers for u16 fields
pub mod u16_direct {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u16, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrU16 {
            String(String),
            U16(u16),
        }

        match StringOrU16::deserialize(deserializer)? {
            StringOrU16::String(s) => s.parse::<u16>().map_err(serde::de::Error::custom),
            StringOrU16::U16(n) => Ok(n),
        }
    }

    pub fn serialize<S>(value: &u16, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }
}
