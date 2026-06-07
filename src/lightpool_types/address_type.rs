use std::fmt::{Display, Formatter, Debug};
use std::str::FromStr;
use std::convert::TryFrom;
use crate::lightpool_types::crypto::PublicKey;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Address([u8; Address::ADDRESS_LENGTH]);

impl Address {
    pub const ADDRESS_LENGTH: usize = 20;

    pub const ZERO: Self = Self([0u8; Self::ADDRESS_LENGTH]);

    pub const MAX: Self = Self([0xff; Self::ADDRESS_LENGTH]);

    pub fn new(bytes: [u8; Self::ADDRESS_LENGTH]) -> Self {
        Self(bytes)
    }

    pub fn zero() -> Self {
        Self([0u8; Self::ADDRESS_LENGTH])
    }

    pub fn from_public_key(public_key: &PublicKey) -> Self {
        use crate::lightpool_types::crypto::Sha512;
        use crate::lightpool_types::crypto::DalekDigest;
        let hash = Sha512::digest(&public_key.as_ref());
        let mut addr_bytes = [0u8; Self::ADDRESS_LENGTH];
        addr_bytes.copy_from_slice(&hash.as_slice()[0..Self::ADDRESS_LENGTH]);
        Self(addr_bytes)
    }

    pub fn as_bytes(&self) -> &[u8; Self::ADDRESS_LENGTH] {
        &self.0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, std::array::TryFromSliceError> {
        let array: [u8; Self::ADDRESS_LENGTH] = bytes.try_into()?;
        Ok(Self(array))
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn encode_base64(&self) -> String {
        base64::encode(&self.0[..])
    }

    pub fn decode_base64(s: &str) -> Result<Self, base64::DecodeError> {
        let bytes = base64::decode(s)?;
        if bytes.len() != Self::ADDRESS_LENGTH {
            return Err(base64::DecodeError::InvalidLength);
        }
        let mut array = [0u8; Self::ADDRESS_LENGTH];
        array.copy_from_slice(&bytes);
        Ok(Self(array))
    }

    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(&self.0))
    }

    #[cfg(feature = "randomization")]
    pub fn random() -> Self {
        use rand::{rngs::OsRng, RngCore};
        let mut bytes = [0u8; Self::ADDRESS_LENGTH];
        OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::zero()
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; Address::ADDRESS_LENGTH]> for Address {
    fn from(bytes: [u8; Self::ADDRESS_LENGTH]) -> Self {
        Self(bytes)
    }
}

impl From<Address> for [u8; Address::ADDRESS_LENGTH] {
    fn from(address: Address) -> Self {
        address.0
    }
}

impl From<u128> for Address {
    fn from(value: u128) -> Self {
        let mut bytes = [0u8; Self::ADDRESS_LENGTH];
        let value_bytes = value.to_be_bytes();
        bytes[Self::ADDRESS_LENGTH - 16..Self::ADDRESS_LENGTH].copy_from_slice(&value_bytes);
        Self::new(bytes)
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = std::array::TryFromSliceError;

    fn try_from(item: &[u8]) -> Result<Self, Self::Error> {
        Ok(Address(item.try_into()?))
    }
}

impl FromStr for Address {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s)?;

        if bytes.len() != Self::ADDRESS_LENGTH {
            return Err(hex::FromHexError::InvalidStringLength);
        }

        let mut array = [0u8; Self::ADDRESS_LENGTH];
        array.copy_from_slice(&bytes);
        Ok(Self::new(array))
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}
