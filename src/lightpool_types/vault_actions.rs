// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use compact_str::CompactString;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVaultParams {
    pub name: CompactString,
    pub quote_token: ContractAddress,
    pub share_name: CompactString,
    pub share_symbol: CompactString,
    pub seed_amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositVaultParams {
    pub amount: u64,
    pub quote_token: ContractAddress,
    pub share_token: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawVaultParams {
    pub shares: u64,
    pub quote_token: ContractAddress,
    pub share_token: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVaultManagerParams {
    pub manager: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetVaultAllowDepositParams {
    pub allow_deposit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloseVaultParams {}
