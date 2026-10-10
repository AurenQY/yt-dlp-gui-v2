//! Stable v2 task identity: Sqids([Unix milliseconds, OS-random u64]).
//! No registry, sequence or collision lookup is involved in generation.
use serde::{Deserialize, Deserializer, Serialize};
use sqids::Sqids;
use std::{
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn encoder() -> &'static Sqids {
    static ENCODER: OnceLock<Sqids> = OnceLock::new();
    ENCODER.get_or_init(|| {
        Sqids::builder()
            .alphabet(ALPHABET.chars().collect())
            .min_length(0)
            .blocklist(Default::default())
            .build()
            .expect("Fixed Sqids configuration")
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct PersistentItemId(String);

impl PersistentItemId {
    pub(crate) fn new() -> Self {
        let timestamp = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System clock must be after Unix epoch")
                .as_millis(),
        )
        .expect("Unix milliseconds fit u64");
        let mut random = [0u8; 8];
        getrandom::fill(&mut random).expect("OS random source is required for persistent item IDs");
        Self::from_parts(timestamp, u64::from_le_bytes(random))
    }

    fn from_parts(timestamp: u64, random: u64) -> Self {
        Self(
            encoder()
                .encode(&[timestamp, random])
                .expect("Two u64 Sqids inputs"),
        )
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn parse(value: String) -> Result<Self, &'static str> {
        // Format/path safety validation is not a collision check.
        if value.is_empty()
            || value.len() > 32
            || !value.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err("Invalid persistent item ID");
        }
        let numbers = encoder().decode(&value);
        if numbers.len() != 2 || encoder().encode(&numbers).ok().as_deref() != Some(&value) {
            return Err("Invalid persistent item ID");
        }
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for PersistentItemId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
