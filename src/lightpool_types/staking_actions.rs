use compact_str::CompactString;
use crate::lightpool_types::contract::ContractAddress;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitStakingConfigParams {
    pub lpl_token: ContractAddress,
    pub min_bond: u64,
    pub committee_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondLplParams {
    pub lpl_token: ContractAddress,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbondLplParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromoteParams {}

impl fmt::Display for InitStakingConfigParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InitStakingConfig(lpl_token: {}, min_bond: {}, committee_size: {})",
            self.lpl_token,
            self.min_bond,
            self.committee_size
        )
    }
}

impl fmt::Display for BondLplParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BondLpl(lpl_token: {}, amount: {})", self.lpl_token, self.amount)
    }
}

impl fmt::Display for UnbondLplParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnbondLpl(amount: {})", self.amount)
    }
}

impl fmt::Display for PromoteParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Promote()")
    }
}
