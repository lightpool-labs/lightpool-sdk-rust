use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::OrderId;
use compact_str::CompactString;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Market state enum representing different operational states
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MarketState {
    Active,
    Paused,
    PostOnly,
    CancelOnly,
    Closed,
    Maintenance,
}

impl fmt::Display for MarketState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarketState::Active => write!(f, "Active"),
            MarketState::Paused => write!(f, "Paused"),
            MarketState::PostOnly => write!(f, "PostOnly"),
            MarketState::CancelOnly => write!(f, "CancelOnly"),
            MarketState::Closed => write!(f, "Closed"),
            MarketState::Maintenance => write!(f, "Maintenance"),
        }
    }
}

/// Preset skip-list depth for a side book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideBookSize {
    Small,
    Middle,
    Large,
}

impl SideBookSize {
    pub const SMALL_SKIP_LEVELS: u8 = 8;
    pub const MIDDLE_SKIP_LEVELS: u8 = 16;
    pub const LARGE_SKIP_LEVELS: u8 = 32;

    pub fn skip_levels(self) -> u8 {
        match self {
            SideBookSize::Small => Self::SMALL_SKIP_LEVELS,
            SideBookSize::Middle => Self::MIDDLE_SKIP_LEVELS,
            SideBookSize::Large => Self::LARGE_SKIP_LEVELS,
        }
    }
}

/// Parameters for creating a new market/trading pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMarketParams {
    pub name: CompactString,
    pub base_token: Address,
    pub quote_token: Address,
    pub min_order_size: u64,
    pub tick_size: u64,
    pub maker_fee_bps: u16,
    pub taker_fee_bps: u16,
    pub allow_market_orders: bool,
    pub state: MarketState,
    pub limit_order: bool,
    pub side_book_size: SideBookSize,
}

/// Market update parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMarketParams {
    pub min_order_size: Option<u64>,
    pub maker_fee_bps: Option<u16>,
    pub taker_fee_bps: Option<u16>,
    pub allow_market_orders: Option<bool>,
    pub state: Option<MarketState>,
}

/// Order side enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Time in force enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC,
    IOC,
    FOK,
}

/// Trigger type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerType {
    TP,
    SL,
}

/// Order type for PlaceOrderParams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderParamsType {
    Limit {
        tif: TimeInForce,
    },
    Market {
        slippage: u64,
    },
    Trigger {
        trigger_price: u64,
        is_market: bool,
        trigger_type: TriggerType,
    },
}

/// Unified order parameters for all order types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOrderParams {
    pub side: OrderSide,
    pub amount: u64,
    pub order_type: OrderParamsType,
    pub limit_price: u64,
}

/// Limit order parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrderParams {
    pub side: OrderSide,
    pub limit_price: u64,
    pub amount: u64,
    pub tif: TimeInForce,
}

/// Trigger order parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerOrderParams {
    pub side: OrderSide,
    pub trigger_price: u64,
    pub limit_price: u64,
    pub amount: u64,
    pub is_market: bool,
    pub trigger_type: TriggerType,
}

/// Market order parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketOrderParams {
    pub side: OrderSide,
    pub amount: u64,
    pub slippage: u64,
    pub limit_price: u64,
}

/// Cancel order parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderParams {
    pub order_id: OrderId,
}

impl fmt::Display for CreateMarketParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CreateMarket(name: {}, min_size: {}, tick_size: {}, fees: {}/{} bps, market_orders: {}, limit_orders: {}, side_book_size: {:?}, state: {})",
            self.name,
            self.min_order_size,
            self.tick_size,
            self.maker_fee_bps,
            self.taker_fee_bps,
            if self.allow_market_orders {
                "allowed"
            } else {
                "not allowed"
            },
            if self.limit_order {
                "allowed"
            } else {
                "not allowed"
            },
            self.side_book_size,
            self.state
        )
    }
}

impl fmt::Display for UpdateMarketParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UpdateMarket(changes: {}{}{}{}{})",
            self.min_order_size
                .map_or(String::new(), |v| format!("min_size: {}, ", v)),
            self.maker_fee_bps
                .map_or(String::new(), |v| format!("maker_fee: {} bps, ", v)),
            self.taker_fee_bps
                .map_or(String::new(), |v| format!("taker_fee: {} bps, ", v)),
            self.allow_market_orders.map_or(String::new(), |v| {
                format!(
                    "market_orders: {}, ",
                    if v { "allowed" } else { "not allowed" }
                )
            }),
            self.state
                .as_ref()
                .map_or(String::new(), |v| format!("state: {}", v))
        )
    }
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "Buy"),
            OrderSide::Sell => write!(f, "Sell"),
        }
    }
}

impl fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeInForce::GTC => write!(f, "GTC"),
            TimeInForce::IOC => write!(f, "IOC"),
            TimeInForce::FOK => write!(f, "FOK"),
        }
    }
}

impl fmt::Display for TriggerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriggerType::TP => write!(f, "TakeProfit"),
            TriggerType::SL => write!(f, "StopLoss"),
        }
    }
}

impl fmt::Display for OrderParamsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderParamsType::Limit { tif } => write!(f, "Limit(tif: {})", tif),
            OrderParamsType::Market { slippage } => {
                write!(f, "Market(slippage: {}bps)", slippage)
            }
            OrderParamsType::Trigger {
                trigger_price,
                is_market,
                trigger_type,
            } => write!(
                f,
                "Trigger(trigger: {}, {}, {})",
                trigger_price,
                if *is_market { "market" } else { "limit" },
                trigger_type
            ),
        }
    }
}

impl fmt::Display for PlaceOrderParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Order({}: {}, price: {}, {})",
            self.side, self.amount, self.limit_price, self.order_type
        )
    }
}

impl fmt::Display for LimitOrderParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LimitOrder({}: {}, price: {}, tif: {})",
            self.side, self.amount, self.limit_price, self.tif
        )
    }
}

impl fmt::Display for TriggerOrderParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TriggerOrder({}: {}, trigger: {}, limit: {}, {}, {})",
            self.side,
            self.amount,
            self.trigger_price,
            self.limit_price,
            if self.is_market { "market" } else { "limit" },
            self.trigger_type
        )
    }
}

impl fmt::Display for MarketOrderParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MarketOrder({}: {}, slippage: {}bps, limit_price: {})",
            self.side, self.amount, self.slippage, self.limit_price
        )
    }
}

impl fmt::Display for CancelOrderParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CancelOrder(order_id: {})", self.order_id)
    }
}
