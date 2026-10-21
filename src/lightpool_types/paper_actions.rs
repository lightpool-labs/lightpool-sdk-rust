// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use compact_str::CompactString;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperCreateMarketParams {
    pub admin: Address,
    pub quote_token: ContractAddress,
    pub paper_name: CompactString,
    pub paper_symbol: CompactString,
    pub win_fee_ppm: u64,
    pub loss_fee_ppm: u64,
    pub staker_share_ppm: u64,
    pub deadband_ppm: u64,
    pub liq_buffer_ppm: u64,
    pub flat_threshold: u64,
    pub flat_rate_ppm: u64,
    pub tail_decay_scale: u64,
    pub staker_cap: u64,
    pub max_oracle_staleness: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperAddInstrumentParams {
    pub name: CompactString,
    pub price_market: ContractAddress,
    pub max_leverage: u64,
    pub min_margin: u64,
    pub min_notional: u64,
    pub max_notional: u64,
    pub long_cap: u64,
    pub short_cap: u64,
    pub base_rate_ppm: u64,
    pub rate_multiplier: u64,
    pub position_multiplier_ppm: u64,
    pub reference_notional: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSetInstrumentParams {
    pub instrument: u32,
    pub long_cap: Option<u64>,
    pub short_cap: Option<u64>,
    pub retired: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSetAmmParams {
    pub amm_pool: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperDepositParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperWithdrawParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperOpenParams {
    pub instrument: u32,
    pub is_long: bool,
    pub margin: u64,
    pub leverage: u64,
    pub fund_from_queued: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperCloseParams {
    pub positions: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperLiquidateParams {
    pub position: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperHarvestParams {
    pub max_entries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperStakeParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperUnstakeParams {
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaperClaimParams {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaperPushFeesParams {}
