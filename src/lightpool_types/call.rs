// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::name_type::Name;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::name;
use compact_str::CompactString;
use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::object::ObjectID;
use crate::lightpool_types::spot_actions::MarketState;
use crate::token_events::format_token_amount;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMarketInfoParams {}

pub const MARKET_INFO_ACTION: Name = name!("mkt_info");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetMarket {
    pub name: CompactString,
    pub base_token: ContractAddress,
    pub quote_token: ContractAddress,
    pub base_balance: ObjectID,
    pub quote_balance: ObjectID,
    pub min_order_size: u64,
    pub tick_size: u64,
    pub maker_fee_bps: u16,
    pub taker_fee_bps: u16,
    pub allow_market_orders: bool,
    pub state: MarketState,
    pub creator: Address,
    pub accrued_quote_fees: u64,
    pub last_price: Option<u64>,
    pub next_order_id: u64,
    pub base_balance_amount: u64,
    pub quote_balance_amount: u64,
}

impl fmt::Display for GetMarket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GetMarket(name: {}, base: {}, quote: {}, base_balance: {}, quote_balance: {}, min_order: {}, tick: {}, fees: {}/{} bps, market_orders: {}, state: {}, creator: {}, accrued_quote_fees: {}, last_price: {:?}, next_order_id: {}, base_amount: {}, quote_amount: {})",
            self.name,
            self.base_token,
            self.quote_token,
            self.base_balance,
            self.quote_balance,
            self.min_order_size,
            self.tick_size,
            self.maker_fee_bps,
            self.taker_fee_bps,
            if self.allow_market_orders { "allowed" } else { "not allowed" },
            self.state,
            self.creator,
            self.accrued_quote_fees,
            self.last_price,
            self.next_order_id,
            self.base_balance_amount,
            self.quote_balance_amount,
        )
    }
}

impl GetMarket {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "base_token": self.base_token.to_string(),
            "quote_token": self.quote_token.to_string(),
            "base_balance": self.base_balance.to_string(),
            "quote_balance": self.quote_balance.to_string(),
            "min_order_size": format_token_amount(self.min_order_size),
            "tick_size": format_token_amount(self.tick_size),
            "maker_fee_bps": self.maker_fee_bps,
            "taker_fee_bps": self.taker_fee_bps,
            "allow_market_orders": self.allow_market_orders,
            "state": format!("{}", self.state),
            "creator": self.creator.to_string(),
            "accrued_quote_fees": format_token_amount(self.accrued_quote_fees),
            "last_price": self.last_price.map(format_token_amount),
            "next_order_id": self.next_order_id,
            "base_balance_amount": format_token_amount(self.base_balance_amount),
            "quote_balance_amount": format_token_amount(self.quote_balance_amount),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOrderBookParams {
    pub depth: u32,
    pub aggregated: bool,
}

pub const ORDER_BOOK_ACTION: Name = name!("ord_book");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetPriceLevel {
    pub price: u64,
    pub total_quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetOrderBook {
    pub best_bids: Vec<GetPriceLevel>,
    pub best_asks: Vec<GetPriceLevel>,
}

impl GetOrderBook {
    pub fn to_json(&self) -> serde_json::Value {
        let bids: Vec<serde_json::Value> = self.best_bids.iter().map(|lvl| serde_json::json!({
            "price": format_token_amount(lvl.price),
            "total_quantity": format_token_amount(lvl.total_quantity),
        })).collect();
        let asks: Vec<serde_json::Value> = self.best_asks.iter().map(|lvl| serde_json::json!({
            "price": format_token_amount(lvl.price),
            "total_quantity": format_token_amount(lvl.total_quantity),
        })).collect();
        serde_json::json!({
            "best_bids": bids,
            "best_asks": asks,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTokenInfoParams {}

pub const TOKEN_INFO_ACTION: Name = name!("token_info");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetTokenInfo {
    pub name: CompactString,
    pub symbol: CompactString,
    pub total_supply: u64,
    pub creator: Address,
    pub mintable: bool,
}

impl GetTokenInfo {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "symbol": self.symbol,
            "total_supply": format_token_amount(self.total_supply),
            "creator": self.creator.to_string(),
            "mintable": self.mintable,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBalanceParams {}

pub const GET_BALANCE_ACTION: Name = name!("get_balance");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetBalance {
    pub total: u64,
    pub locked: u64,
    pub available: u64,
}

impl GetBalance {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "total": format_token_amount(self.total),
            "locked": format_token_amount(self.locked),
            "available": format_token_amount(self.available),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetVaultPortfolioParams {}

pub const VAULT_PORTFOLIO_ACTION: Name = name!("get_pf");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetVaultPortfolioAsset {
    pub market: ContractAddress,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetVaultPortfolio {
    pub account: Address,
    pub assets: Vec<GetVaultPortfolioAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetPaperMarketParams {}

pub const PAPER_GET_MARKET_ACTION: Name = name!("get_mkt");

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetPaperAccountParams {}

pub const PAPER_GET_ACCOUNT_ACTION: Name = name!("get_acct");

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetPaperLpParams {}

pub const PAPER_GET_LP_ACTION: Name = name!("get_lp");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPaperPositionParams {
    pub position: u64,
}

pub const PAPER_GET_POSITION_ACTION: Name = name!("get_pos");

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetPaperQueueParams {}

pub const PAPER_GET_QUEUE_ACTION: Name = name!("get_queue");

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetPaperStakeParams {}

pub const PAPER_GET_STAKE_ACTION: Name = name!("get_stake");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetPaperInstrument {
    pub id: u32,
    pub name: CompactString,
    pub price_market: ContractAddress,
    pub max_leverage: u64,
    pub min_margin: u64,
    pub min_notional: u64,
    pub max_notional: u64,
    pub long_cap: u64,
    pub short_cap: u64,
    pub long_oi: u64,
    pub short_oi: u64,
    pub retired: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetPaperMarket {
    pub admin: Address,
    pub quote_token: ContractAddress,
    pub paper_token: Option<ContractAddress>,
    pub amm_pool: Option<ContractAddress>,
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
    pub next_position_index: u64,
    pub instruments: Vec<GetPaperInstrument>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetPaperAccount {
    pub available: u64,
    pub queued: u64,
    pub entry_head: u64,
    pub entry_tail: u64,
    pub queued_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetPaperLp {
    pub free_cash: u64,
    pub side_bucket: u64,
    pub tracked_lp: i128,
    pub tail_hwm: u64,
    pub queue_head: u64,
    pub queue_tail: u64,
    pub total_queued: u64,
    pub staker_fee_acc: u64,
    pub dev_fee_acc: u64,
    pub buyback_cash: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetPaperPosition {
    pub index: u64,
    pub owner: Address,
    pub instrument: u32,
    pub is_long: bool,
    pub margin: u64,
    pub notional: u64,
    pub entry_price: u64,
    pub bust_price: u64,
    pub funded_from_queued: bool,
    pub opened_at: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetPaperQueue {
    pub queue_head: u64,
    pub queue_tail: u64,
    pub total_queued: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetPaperStake {
    pub total_staked: u64,
    pub acc_reward_per_share: u128,
    pub reward_pool: u64,
    pub pending_dev_fee: u64,
    pub staked: u64,
    pub reward_debt: u128,
    pub claimable: u64,
    pub pending_reward: u64,
}
