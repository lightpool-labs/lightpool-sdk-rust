// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::contract::ContractAddress;
use lightpool_crypto::PublicKey;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum StakePurpose {
    Committee = 1,
    MarketOperator = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitStakingConfigParams {
    pub lpl_token: ContractAddress,
    pub min_bond: u64,
    pub committee_size: u32,
    #[serde(default)]
    pub unbonding_period_blocks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondLplParams {
    pub lpl_token: ContractAddress,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbondLplParams {
    pub lpl_token: ContractAddress,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocateStakeParams {
    pub purpose: StakePurpose,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeallocateStakeParams {
    pub purpose: StakePurpose,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawUnbondParams {
    pub lpl_token: ContractAddress,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterValidatorParams {
    pub consensus_pubkey: PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromoteParams {}

impl fmt::Display for InitStakingConfigParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InitStakingConfig(lpl_token: {}, min_bond: {}, committee_size: {}, unbonding_period_blocks: {})",
            self.lpl_token,
            self.min_bond,
            self.committee_size,
            self.unbonding_period_blocks
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
        write!(
            f,
            "UnbondLpl(lpl_token: {}, amount: {})",
            self.lpl_token, self.amount
        )
    }
}

impl fmt::Display for AllocateStakeParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AllocateStake(purpose: {:?}, amount: {})",
            self.purpose, self.amount
        )
    }
}

impl fmt::Display for DeallocateStakeParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DeallocateStake(purpose: {:?}, amount: {})",
            self.purpose, self.amount
        )
    }
}

impl fmt::Display for WithdrawUnbondParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WithdrawUnbond(lpl_token: {}, amount: {})",
            self.lpl_token, self.amount
        )
    }
}

impl fmt::Display for RegisterValidatorParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RegisterValidator(consensus_pubkey: {:?})",
            self.consensus_pubkey
        )
    }
}

impl fmt::Display for PromoteParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Promote()")
    }
}
