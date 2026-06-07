use std::fmt;

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::module::Module;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ContractAddress([u8; ContractAddress::CONTRACT_ADDRESS_LENGTH]);

impl ContractAddress {
    pub const CONTRACT_ADDRESS_LENGTH: usize = 8;

    pub const ZERO: Self = Self([0u8; Self::CONTRACT_ADDRESS_LENGTH]);

    pub fn new(module: Module, rest: [u8; 7]) -> Self {
        let mut bytes = [0u8; Self::CONTRACT_ADDRESS_LENGTH];
        bytes[0] = module.as_u8();
        bytes[1..Self::CONTRACT_ADDRESS_LENGTH].copy_from_slice(&rest);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; Self::CONTRACT_ADDRESS_LENGTH]) -> Self {
        Self(bytes)
    }

    pub fn module(&self) -> Module {
        Module::from(self.0[0])
    }

    pub fn rest(&self) -> [u8; 7] {
        let mut rest = [0u8; 7];
        rest.copy_from_slice(&self.0[1..Self::CONTRACT_ADDRESS_LENGTH]);
        rest
    }

    pub fn as_bytes(&self) -> &[u8; Self::CONTRACT_ADDRESS_LENGTH] {
        &self.0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn to_address(self) -> Address {
        let mut bytes = [0u8; Address::ADDRESS_LENGTH];
        bytes[..Self::CONTRACT_ADDRESS_LENGTH].copy_from_slice(self.as_bytes());
        Address::new(bytes)
    }
}

impl fmt::Display for ContractAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x")?;
        for byte in self.0.iter() {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl fmt::Debug for ContractAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContractAddress(module: {}, rest: 0x", self.module())?;
        for byte in self.rest() {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, ")")
    }
}

impl From<[u8; ContractAddress::CONTRACT_ADDRESS_LENGTH]> for ContractAddress {
    fn from(bytes: [u8; ContractAddress::CONTRACT_ADDRESS_LENGTH]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<ContractAddress> for [u8; ContractAddress::CONTRACT_ADDRESS_LENGTH] {
    fn from(address: ContractAddress) -> Self {
        address.0
    }
}
