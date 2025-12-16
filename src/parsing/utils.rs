use serde::{Deserialize, Deserializer, de::Error};
use std::str::FromStr;

/// Deserializes a string into a u64
pub fn deserialize_str_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    u64::from_str(&s).map_err(Error::custom)
}
