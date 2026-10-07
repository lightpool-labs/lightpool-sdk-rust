// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::contract::ContractAddress;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmCreatePoolParams {
    pub token0: ContractAddress,
    pub token1: ContractAddress,
    pub fee: u32,
    pub tick_spacing: i32,
    pub sqrt_price_x96: u128,
    pub whitelist: Option<ContractAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmAddLiquidityParams {
    pub token0: ContractAddress,
    pub token1: ContractAddress,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub whitelist: Option<ContractAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmRemoveLiquidityParams {
    pub token0: ContractAddress,
    pub token1: ContractAddress,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub whitelist: Option<ContractAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmSwapParams {
    pub token0: ContractAddress,
    pub token1: ContractAddress,
    pub zero_for_one: bool,
    pub amount: u64,
    pub sqrt_price_limit_x96: u128,
    pub whitelist: Option<ContractAddress>,
}
