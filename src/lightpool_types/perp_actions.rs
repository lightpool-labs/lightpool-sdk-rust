use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::object::ObjectID;
use compact_str::CompactString;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Market state enum representing different operational states
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MarketState {
    /// Market is active and fully operational
    Active,
    /// Market is paused - no new orders, but cancellations allowed
    Paused,
    /// Market is in post-only mode - only limit orders with post-only flag allowed
    PostOnly,
    /// Market is in cancel-only mode - only cancellations allowed, no new orders
    CancelOnly,
    /// Market is closed - no operations allowed
    Closed,
    /// Market is in maintenance mode - administrative operations only
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

/// Order side enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Position side enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
}

/// Scale level for scale orders
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScaleLevel {
    /// Price level for this scale
    pub price: u64,
    /// Size to execute at this price level
    pub size: u64,
    /// Whether this level has been executed
    pub executed: bool,
}

impl ScaleLevel {
    pub fn new(price: u64, size: u64) -> Self {
        Self {
            price,
            size,
            executed: false,
        }
    }
}

/// Time in Force types for limit orders
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TimeInForce {
    /// Good Till Canceled - order remains active until canceled
    GTC,
    /// Immediate Or Cancel - execute immediately what can be filled, cancel the rest
    IOC,
    /// Fill Or Kill - execute the entire order immediately or cancel it
    FOK,
    /// Post Only - only add liquidity, never take liquidity
    PostOnly,
}

impl fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeInForce::GTC => write!(f, "GTC"),
            TimeInForce::IOC => write!(f, "IOC"),
            TimeInForce::FOK => write!(f, "FOK"),
            TimeInForce::PostOnly => write!(f, "PostOnly"),
        }
    }
}

/// Order type for perpetual trading (based on Hyperliquid model)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OrderType {
    /// Limit order with time in force
    Limit {
        time_in_force: TimeInForce,
    },
    /// Market order with slippage protection
    Market {
        slippage: u64,
    },
    /// Scale order - executes at multiple price levels
    Scale {
        scale_levels: Vec<ScaleLevel>,
        time_in_force: TimeInForce,
    },
    /// Stop limit order - limit order triggered by stop price
    StopLimit {
        stop_price: u64,
        limit_price: u64,
    },
    /// Stop market order - market order triggered by stop price
    StopMarket {
        stop_price: u64,
    },
    /// TWAP order - Time Weighted Average Price execution
    TWAP {
        start_time: u64,
        end_time: u64,
        total_size: u64,
        chunk_size: u64,
        chunk_interval: u64,
    },
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderType::Limit { time_in_force } => {
                write!(f, "Limit({})", time_in_force)
            }
            OrderType::Market { slippage } => {
                write!(f, "Market(slippage: {})", slippage)
            }
            OrderType::Scale { scale_levels, time_in_force } => {
                write!(f, "Scale({} levels, {})", scale_levels.len(), time_in_force)
            }
            OrderType::StopLimit { stop_price, limit_price } => {
                write!(f, "StopLimit(stop: {}, limit: {})", stop_price, limit_price)
            }
            OrderType::StopMarket { stop_price } => {
                write!(f, "StopMarket(stop: {})", stop_price)
            }
            OrderType::TWAP { start_time, end_time, total_size, chunk_size, chunk_interval } => {
                write!(f, "TWAP({}->{}, size: {}, chunk: {} every {})", 
                    start_time, end_time, total_size, chunk_size, chunk_interval)
            }
        }
    }
}

/// Leverage mode enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeverageMode {
    Cross,   // Cross margin mode
    Isolated, // Isolated margin mode
}

/// Trigger type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    TP, // Take Profit
    SL, // Stop Loss
}

/// Trigger parameters for take profit and stop loss
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub take_profit: Option<u64>, // Take profit price
    pub stop_loss: Option<u64>,   // Stop loss price
}

// Use shared OrderId and OrderIdType from lightpool_types
use crate::lightpool_types::{OrderId, OrderIdType};

/// Parameters for creating a new perpetual market/trading pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMarketParams {
    pub name: CompactString,           // Market name (e.g., "BTC/USDC")
    pub base_token: Address,           // Base token address
    pub collateral_token: Address,     // Collateral token address (e.g., USDC)
    pub min_order_size: u64,           // Minimum order size in base token
    pub tick_size: u64,                // Minimum price increment
    pub maker_fee_bps: u16,            // Maker fee in basis points (1/10000)
    pub taker_fee_bps: u16,            // Taker fee in basis points (1/10000)
    pub allow_market_orders: bool,     // Whether market orders are allowed
    pub state: MarketState,            // Initial market state
    pub limit_order: bool,             // Whether limit orders are allowed
    pub max_leverage: u32,             // Maximum leverage allowed for this market
    pub maintenance_margin_bps: u16,   // Maintenance margin requirement in basis points
    pub initial_margin_bps: u16,       // Initial margin requirement in basis points
    pub funding_interval: u64,         // Funding interval in seconds (e.g., 3600 for hourly)
    pub liquidation_fee_bps: u16,      // Liquidation fee in basis points (e.g., 50 for 0.5%)
    pub max_position_size: u64,        // Maximum position size allowed (e.g., 1000000)
    pub max_price_deviation_bps: u16,  // Maximum price deviation in basis points (e.g., 500 for 5%)
    pub max_funding_rate_bps: u16,     // Maximum funding rate in basis points (e.g., 1000 for 10%)
}

/// Market update parameters for perpetual markets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMarketParams {
    pub min_order_size: Option<u64>,   // New minimum order size (if changing)
    pub maker_fee_bps: Option<u16>,    // New maker fee (if changing)
    pub taker_fee_bps: Option<u16>,    // New taker fee (if changing)
    pub allow_market_orders: Option<bool>, // New market orders setting (if changing)
    pub state: Option<MarketState>,    // New market state (if changing)
    pub max_leverage: Option<u32>,     // New maximum leverage (if changing)
    pub maintenance_margin_bps: Option<u16>, // New maintenance margin requirement (if changing)
    pub initial_margin_bps: Option<u16>,     // New initial margin requirement (if changing)
    pub funding_interval: Option<u64>,       // New funding interval in seconds (if changing)
    pub liquidation_fee_bps: Option<u16>,    // New liquidation fee in basis points (if changing)
    pub max_position_size: Option<u64>,      // New maximum position size allowed (if changing)
    pub max_price_deviation_bps: Option<u16>, // New maximum price deviation in basis points (if changing)
    pub max_funding_rate_bps: Option<u16>,    // New maximum funding rate in basis points (if changing)
}

/// Market open position parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketOpenParams {
    pub position_side: PositionSide, // Long or Short
    pub size: u64,               // Size in base token
    pub order_type: OrderType,   // Type of order with specific parameters
    pub limit_price: u64,        // Limit price for all order types
    pub leverage_mode: LeverageMode, // Cross or Isolated margin
    pub leverage: u16,           // Leverage multiplier (e.g., 10x = 10_000)
    pub collateral: Option<u64>, // Collateral amount for isolated margin
    pub trigger: Option<Trigger>, // Take profit and stop loss
    pub client_order_id: Option<CompactString>, // Client order ID for external reference
    pub position_id: Option<ObjectID>, // Position ID to update (None for new position)
}

/// Market close position parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCloseParams {
    pub size: u64,               // Size in base token to close
    pub order_type: OrderType,   // Type of order with specific parameters
    pub limit_price: u64,        // Limit price for all order types
    pub trigger: Option<Trigger>, // Take profit and stop loss
    pub client_order_id: Option<CompactString>, // Client order ID for external reference
    pub position_id: ObjectID,   // Position ID to close
}

/// Cancel order parameters for perpetual markets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderParams {
    pub order_id: OrderId,      // ID of the order to cancel
}

// Display implementations for better logging
impl fmt::Display for CreateMarketParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CreatePerpMarket(name: {}, min_size: {}, tick_size: {}, fees: {}/{} bps, market_orders: {}, limit_orders: {}, state: {}, max_leverage: {}x, margins: {}/{} bps, funding_interval: {}s, liquidation_fee: {} bps, max_position: {}, max_deviation: {} bps, max_funding_rate: {} bps)", 
               self.name, self.min_order_size, self.tick_size, 
               self.maker_fee_bps, self.taker_fee_bps,
               if self.allow_market_orders { "allowed" } else { "not allowed" },
               if self.limit_order { "allowed" } else { "not allowed" },
               self.state, self.max_leverage, self.initial_margin_bps, self.maintenance_margin_bps,
               self.funding_interval, self.liquidation_fee_bps, self.max_position_size,
               self.max_price_deviation_bps, self.max_funding_rate_bps)
    }
}

impl fmt::Display for UpdateMarketParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UpdatePerpMarket(changes: {}{}{}{}{}{}{}{}{}{}{}{}{}", 
               self.min_order_size.map_or("".to_string(), |v| format!("min_size: {}, ", v)),
               self.maker_fee_bps.map_or("".to_string(), |v| format!("maker_fee: {} bps, ", v)),
               self.taker_fee_bps.map_or("".to_string(), |v| format!("taker_fee: {} bps, ", v)),
               self.allow_market_orders.map_or("".to_string(), |v| format!("market_orders: {}, ", if v { "allowed" } else { "not allowed" })),
               self.state.as_ref().map_or("".to_string(), |v| format!("state: {}, ", v)),
               self.max_leverage.map_or("".to_string(), |v| format!("max_leverage: {}x, ", v)),
               self.maintenance_margin_bps.map_or("".to_string(), |v| format!("maintenance_margin: {} bps, ", v)),
               self.initial_margin_bps.map_or("".to_string(), |v| format!("initial_margin: {} bps, ", v)),
               self.funding_interval.map_or("".to_string(), |v| format!("funding_interval: {}s, ", v)),
               self.liquidation_fee_bps.map_or("".to_string(), |v| format!("liquidation_fee: {} bps, ", v)),
               self.max_position_size.map_or("".to_string(), |v| format!("max_position: {}, ", v)),
               self.max_price_deviation_bps.map_or("".to_string(), |v| format!("max_deviation: {} bps, ", v)),
               self.max_funding_rate_bps.map_or("".to_string(), |v| format!("max_funding_rate: {} bps", v)))
    }
}

impl fmt::Display for MarketOpenParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let position_info = self.position_id.map_or("new position".to_string(), |id| format!("position: {}", id));
        write!(f, "MarketOpen({}: {}, price: {}, leverage: {}x, {}, {})", 
               self.position_side,
               self.size, 
               self.limit_price,
               self.leverage,
               self.order_type,
               position_info)
    }
}

impl fmt::Display for MarketCloseParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MarketClose({}, price: {}, {}, position: {})", 
               self.size, 
               self.limit_price,
               self.order_type,
               self.position_id)
    }
}

impl fmt::Display for CancelOrderParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CancelPerpOrder(order_id: {})", self.order_id)
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

impl fmt::Display for PositionSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PositionSide::Long => write!(f, "Long"),
            PositionSide::Short => write!(f, "Short"),
        }
    }
}

impl fmt::Display for LeverageMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LeverageMode::Cross => write!(f, "Cross"),
            LeverageMode::Isolated => write!(f, "Isolated"),
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

impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tp_str = self.take_profit.map_or("None".to_string(), |p| format!("{}", p));
        let sl_str = self.stop_loss.map_or("None".to_string(), |p| format!("{}", p));
        write!(f, "Trigger(TP: {}, SL: {})", tp_str, sl_str)
    }
} 