use std::fmt;
use std::str::FromStr;

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// ObjectID is the unique identifier for an object in the system.
/// Layout: [0:8] ContractAddress, [8:32] 24-byte module-specific payload
pub struct ObjectID([u8; ObjectID::OBJECT_ID_LENGTH]);

impl ObjectID {
    pub const OBJECT_ID_LENGTH: usize = 32;

    pub const CONTRACT_ADDRESS_OFFSET: usize = 0;
    pub const CONTRACT_ADDRESS_LENGTH: usize = ContractAddress::CONTRACT_ADDRESS_LENGTH;

    pub const PAYLOAD_OFFSET: usize = Self::CONTRACT_ADDRESS_OFFSET + Self::CONTRACT_ADDRESS_LENGTH;
    pub const PAYLOAD_LENGTH: usize = 24;

    pub const ADDRESS_OFFSET: usize = Self::PAYLOAD_OFFSET;
    pub const ADDRESS_LENGTH: usize = Address::ADDRESS_LENGTH;

    pub const POSITION_OFFSET: usize = Self::ADDRESS_OFFSET + Self::ADDRESS_LENGTH;
    pub const POSITION_LENGTH: usize = 4;

    pub const ZERO: Self = Self([0u8; Self::OBJECT_ID_LENGTH]);

    pub const MAX: Self = Self([0xff; Self::OBJECT_ID_LENGTH]);

    pub fn payload_from_address_position(address: Address, position: u32) -> [u8; Self::PAYLOAD_LENGTH] {
        let mut payload = [0u8; Self::PAYLOAD_LENGTH];
        payload[0..Self::ADDRESS_LENGTH].copy_from_slice(address.as_slice());
        payload[Self::ADDRESS_LENGTH..Self::PAYLOAD_LENGTH]
            .copy_from_slice(&position.to_be_bytes());
        payload
    }

    pub fn generate(contract: ContractAddress, payload: [u8; Self::PAYLOAD_LENGTH]) -> Self {
        let mut bytes = [0u8; Self::OBJECT_ID_LENGTH];
        bytes[Self::CONTRACT_ADDRESS_OFFSET..Self::PAYLOAD_OFFSET]
            .copy_from_slice(contract.as_bytes());
        bytes[Self::PAYLOAD_OFFSET..Self::OBJECT_ID_LENGTH]
            .copy_from_slice(&payload);
        Self(bytes)
    }

    pub fn contract_address(&self) -> ContractAddress {
        let mut bytes = [0u8; ContractAddress::CONTRACT_ADDRESS_LENGTH];
        bytes.copy_from_slice(
            &self.0[Self::CONTRACT_ADDRESS_OFFSET..Self::ADDRESS_OFFSET],
        );
        ContractAddress::from_bytes(bytes)
    }

    pub fn address(&self) -> Address {
        let bytes: [u8; Address::ADDRESS_LENGTH] = self.0
            [Self::ADDRESS_OFFSET..Self::POSITION_OFFSET]
            .try_into()
            .expect("address slice is 20 bytes");
        Address::new(bytes)
    }

    pub fn position(&self) -> u32 {
        u32::from_be_bytes(
            self.0[Self::POSITION_OFFSET..Self::OBJECT_ID_LENGTH]
                .try_into()
                .expect("position slice is 4 bytes"),
        )
    }

    pub fn new(value: [u8; Self::OBJECT_ID_LENGTH]) -> Self {
        Self(value)
    }

    pub fn as_bytes(&self) -> &[u8; Self::OBJECT_ID_LENGTH] {
        &self.0
    }

    pub fn from_string(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let s = s.strip_prefix("0x").unwrap_or(s);

        let padded = format!("{:0>64}", s);

        let mut result = [0u8; Self::OBJECT_ID_LENGTH];
        for i in 0..Self::OBJECT_ID_LENGTH {
            let start = i * 2;
            let end = start + 2;
            result[i] = u8::from_str_radix(&padded[start..end], 16)?;
        }

        Ok(Self(result))
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    #[cfg(feature = "randomization")]
    pub fn random() -> Self {
        use rand::{rngs::OsRng, RngCore};
        let mut bytes = [0u8; Self::OBJECT_ID_LENGTH];
        OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }
}

impl FromStr for ObjectID {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s)
    }
}

impl fmt::Display for ObjectID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x")?;
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl fmt::Debug for ObjectID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ObjectID(contract: {}, address: {}, position: {})",
            self.contract_address(),
            self.address(),
            self.position(),
        )
    }
}

impl From<[u8; 32]> for ObjectID {
    fn from(value: [u8; 32]) -> Self {
        Self::new(value)
    }
}

impl From<ObjectID> for [u8; 32] {
    fn from(id: ObjectID) -> Self {
        id.0
    }
}
