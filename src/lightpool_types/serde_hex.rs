// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::crypto::Digest;

fn encode_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(2 + bytes.len() * 2);
    out.push_str("0x");
    out.push_str(&hex::encode(bytes));
    out
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    let hex = value
        .trim()
        .strip_prefix("0x")
        .or_else(|| value.trim().strip_prefix("0X"))
        .unwrap_or(value.trim());
    if hex.is_empty() {
        return Ok(Vec::new());
    }
    hex::decode(hex).map_err(|e| format!("invalid hex: {e}"))
}

fn bytes_to_array<const N: usize>(bytes: &[u8]) -> Result<[u8; N], String> {
    bytes
        .try_into()
        .map_err(|_| format!("expected {N} bytes, got {}", bytes.len()))
}

#[derive(Deserialize)]
#[serde(untagged)]
enum HexOrBytes {
    Hex(String),
    Bytes(Vec<u8>),
}

impl HexOrBytes {
    fn into_vec(self) -> Result<Vec<u8>, String> {
        match self {
            Self::Hex(value) => decode_hex(&value),
            Self::Bytes(bytes) => Ok(bytes),
        }
    }
}

pub mod bytes {
    use super::*;

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            encode_hex(bytes).serialize(serializer)
        } else {
            bytes.serialize(serializer)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        if deserializer.is_human_readable() {
            HexOrBytes::deserialize(deserializer)?
                .into_vec()
                .map_err(serde::de::Error::custom)
        } else {
            Vec::<u8>::deserialize(deserializer)
        }
    }
}

pub mod digest {
    use super::*;

    pub fn serialize<S: Serializer>(digest: &Digest, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            digest.to_hex().serialize(serializer)
        } else {
            digest.as_bytes().serialize(serializer)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Digest, D::Error> {
        if deserializer.is_human_readable() {
            let bytes = HexOrBytes::deserialize(deserializer)?
                .into_vec()
                .map_err(serde::de::Error::custom)?;
            Ok(Digest::new(
                bytes_to_array(&bytes).map_err(serde::de::Error::custom)?,
            ))
        } else {
            Ok(Digest::new(<[u8; 32]>::deserialize(deserializer)?))
        }
    }
}

pub mod option_digest {
    use super::*;

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Digest>, D::Error> {
        let Some(raw) = Option::<HexOrBytes>::deserialize(deserializer)? else {
            return Ok(None);
        };
        let bytes = raw.into_vec().map_err(serde::de::Error::custom)?;
        Ok(Some(Digest::new(
            bytes_to_array(&bytes).map_err(serde::de::Error::custom)?,
        )))
    }
}

pub mod address {
    use super::*;

    pub fn serialize<S: Serializer>(address: &Address, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            address.to_hex().serialize(serializer)
        } else {
            address.as_bytes().serialize(serializer)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Address, D::Error> {
        if deserializer.is_human_readable() {
            let bytes = HexOrBytes::deserialize(deserializer)?
                .into_vec()
                .map_err(serde::de::Error::custom)?;
            Ok(Address::new(
                bytes_to_array(&bytes).map_err(serde::de::Error::custom)?,
            ))
        } else {
            Ok(Address::new(<[u8; Address::ADDRESS_LENGTH]>::deserialize(
                deserializer,
            )?))
        }
    }
}

pub mod option_address {
    use super::*;

    pub fn serialize<S: Serializer>(
        value: &Option<Address>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(address) if serializer.is_human_readable() => {
                serializer.serialize_some(&address.to_hex())
            }
            Some(address) => serializer.serialize_some(address.as_bytes()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Address>, D::Error> {
        let Some(raw) = Option::<HexOrBytes>::deserialize(deserializer)? else {
            return Ok(None);
        };
        let bytes = raw.into_vec().map_err(serde::de::Error::custom)?;
        Ok(Some(Address::new(
            bytes_to_array(&bytes).map_err(serde::de::Error::custom)?,
        )))
    }
}

pub mod option_contract {
    use super::*;

    pub fn serialize<S: Serializer>(
        value: &Option<ContractAddress>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(contract) if serializer.is_human_readable() => {
                serializer.serialize_some(&contract.to_string())
            }
            Some(contract) => serializer.serialize_some(contract.as_bytes()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<ContractAddress>, D::Error> {
        let Some(raw) = Option::<HexOrBytes>::deserialize(deserializer)? else {
            return Ok(None);
        };
        let bytes = raw.into_vec().map_err(serde::de::Error::custom)?;
        Ok(Some(ContractAddress::from_bytes(
            bytes_to_array(&bytes).map_err(serde::de::Error::custom)?,
        )))
    }
}
