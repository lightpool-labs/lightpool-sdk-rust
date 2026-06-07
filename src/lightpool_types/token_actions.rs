use crate::lightpool_types::address_type::Address;
use compact_str::CompactString;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenParams {
    pub name: CompactString,
    pub symbol: CompactString,
    pub total_supply: u64,
    pub mintable: bool,
    pub to: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintParams {
    pub amount: u64,
    pub to: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferParams {
    pub to: Address,
    pub amount: u64,
}

impl fmt::Display for CreateTokenParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CreateToken(name: {}, symbol: {}, total_supply: {}, mintable: {}, to: {})",
               self.name, self.symbol, self.total_supply, self.mintable, self.to)
    }
}

impl fmt::Display for MintParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mint(amount: {}, to: {})",
               self.amount, self.to)
    }
}

impl fmt::Display for TransferParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Transfer(to: {}, amount: {})",
               self.to, self.amount)
    }
}
