// Copyright (c) LightPool Labs
// Author: xiaoyu1998

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::lightpool_types::contract::ContractAddress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePoolParams {
    pub token: ContractAddress,
    pub max_ltv_bps: u64,
    pub maint_bps: u64,
    pub liq_bonus_bps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawSupplyParams {
    pub shares: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMarginParams {
    pub pool: ContractAddress,
    /// 0 = Cross, 1 = Isolated
    pub mode: u8,
    pub market: Option<ContractAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositCollateralParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawCollateralParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepayParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidateParams {
    pub repay_amount: u64,
}

pub const MARGIN_MODE_CROSS: u8 = 0;
pub const MARGIN_MODE_ISOLATED: u8 = 1;
