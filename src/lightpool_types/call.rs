use crate::lightpool_types::name_type::Name;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::name;
use compact_str::CompactString;
use crate::lightpool_types::address_type::Address;
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
    pub base_token: Address,
    pub quote_token: Address,
    pub base_balance: ObjectID,
    pub quote_balance: ObjectID,
    pub min_order_size: u64,
    pub tick_size: u64,
    pub maker_fee_bps: u16,
    pub taker_fee_bps: u16,
    pub allow_market_orders: bool,
    pub state: MarketState,
    pub creator: Address,
    pub last_price: Option<u64>,
    pub next_order_id: u64,
    pub base_balance_amount: u64,
    pub quote_balance_amount: u64,
}

impl fmt::Display for GetMarket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GetMarket(name: {}, base: {}, quote: {}, base_balance: {}, quote_balance: {}, min_order: {}, tick: {}, fees: {}/{} bps, market_orders: {}, state: {}, creator: {}, last_price: {:?}, next_order_id: {}, base_amount: {}, quote_amount: {})",
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
