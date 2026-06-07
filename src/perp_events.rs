use serde::{Serialize, Deserialize};
use compact_str::CompactString;
use crate::lightpool_types::{Address, ObjectID, TransactionReceipt, EventType, EventData};

/// Order side enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
    Open,
    Close,
}

/// Position side enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
}

/// Leverage mode enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LeverageMode {
    Cross,   // Cross margin mode
    Isolated, // Isolated margin mode
}

/// Order type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Limit,
    Market,
    Trigger,
}

/// Time in force enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC, // Good Till Cancelled
    IOC, // Immediate Or Cancel
    FOK, // Fill Or Kill
    PostOnly,
}

/// Trigger parameters for take profit and stop loss
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub take_profit: Option<u64>, // Take profit price
    pub stop_loss: Option<u64>,   // Stop loss price
}

/// Perpetual order type for OrderCreatedEvent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerpOrderEventType {
    /// Limit order details
    Limit {
        time_in_force: TimeInForce,
    },
    /// Market order details
    Market {
        slippage: u64,
    },
    /// Trigger order details
    Trigger {
        trigger_price: u64,
        limit_price: u64,
        is_market: bool,
        trigger_type: TriggerType,
    },
}

/// Trigger type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    TP, // Take Profit
    SL, // Stop Loss
}

/// Transfer event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEvent {
    pub token: Address,
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub original_balance_id: ObjectID,
    pub to_balance_id: ObjectID,
    pub remainder: u64,
    pub remainder_id: Option<ObjectID>,
}

/// Helper function to format token amount with decimals
pub fn format_token_amount(amount: u64) -> String {
    let whole = amount / 1_000_000;
    let fraction = amount % 1_000_000;
    if fraction == 0 {
        format!("{}", whole)
    } else {
        format!("{}.{:06}", whole, fraction)
    }
}

/// Helper function to format signed token amount with decimals
pub fn format_iamount(amount: i64) -> String {
    let sign = if amount < 0 { "-" } else { "" };
    let abs_amount = amount.abs() as u64;
    let whole = abs_amount / 1_000_000;
    let fraction = abs_amount % 1_000_000;
    if fraction == 0 {
        format!("{}{}", sign, whole)
    } else {
        format!("{}{}.{:06}", sign, whole, fraction)
    }
}

// Use shared OrderId and OrderIdType from lightpool_types
use crate::lightpool_types::{OrderId, OrderIdType};

/// Event emitted when a perpetual market is created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpMarketCreatedEvent {
    pub market_id: ObjectID,
    pub market_address: Address,
    pub name: CompactString,
    pub base_token: Address,
    pub collateral_token: Address,
    pub margin_vault: ObjectID,
    pub price_index_id: ObjectID,
    pub oracle_price_feed_id: ObjectID,
    pub min_order_size: u64,
    pub tick_size: u64,
    pub maker_fee_bps: i16,
    pub taker_fee_bps: i16,
    pub allow_market_orders: bool,
    pub max_leverage: u16,
    pub initial_margin_bps: u16,
    pub maintenance_margin_bps: u16,
    pub liquidation_fee_bps: u16,
    pub max_position_size: u64,
    pub max_price_deviation_bps: u16,
    pub max_funding_rate_bps: u16,
    pub creator: Address,
}

/// Event emitted when a perpetual market is updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpMarketUpdatedEvent {
    pub market_id: ObjectID,
    pub min_order_size: Option<u64>,
    pub tick_size: Option<u64>,
    pub maker_fee_bps: Option<i16>,
    pub taker_fee_bps: Option<i16>,
    pub allow_market_orders: Option<bool>,
    pub max_leverage: Option<u16>,
    pub initial_margin_bps: Option<u16>,
    pub maintenance_margin_bps: Option<u16>,
    pub liquidation_fee_bps: Option<u16>,
    pub max_position_size: Option<u64>,
    pub max_price_deviation_bps: Option<u16>,
    pub max_funding_rate_bps: Option<u16>,
    pub updater: Address,
}

/// Event emitted when a perpetual order is created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpOrderCreatedEvent {
    pub order_id: OrderId,
    pub market: Address,
    pub side: OrderSide,
    pub position_side: PositionSide,
    pub order_type: PerpOrderEventType,
    pub size: u64,
    pub filled_size: u64,
    pub limit_price: u64,
    pub leverage_mode: LeverageMode,
    pub leverage: u16,
    pub collateral: Option<u64>,
    pub reduce_only: bool,
    pub trigger: Option<Trigger>,
    pub creator: Address,
}

/// Event emitted when a perpetual order is filled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpOrderFilledEvent {
    pub order_id: OrderId,
    pub market: Address,
    pub side: OrderSide,
    pub position_side: PositionSide,
    pub filled_price: u64,
    pub filled_amount: u64,
    pub remaining_amount: u64,
    pub is_complete: bool,
    pub fee: u64,
}

/// Perpetual trade type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerpTradeType {
    /// Opening a new position
    Open,
    /// Closing part of position
    Close,
    /// Adding to existing position
    Add,
    /// Reducing existing position
    Reduce,
    /// Liquidation trade
    Liquidation,
}

/// Event emitted when a perpetual position is opened
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpPositionOpenEvent {
    pub position_id: ObjectID,
    pub market: Address,
    pub owner: Address,
    pub side: PositionSide,
    pub size: u64,
    pub entry_price: u64,
    pub leverage_mode: LeverageMode,
    pub leverage: u16,
    pub collateral: u64,
    pub initial_margin: u64,
    pub maintenance_margin: u64,
    pub entry_funding_rate: i16,
}

/// Event emitted when a perpetual position is updated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpPositionUpdatedEvent {
    pub position_id: ObjectID,
    pub market: Address,
    pub owner: Address,
    pub old_size: i64,
    pub new_size: i64,
    pub trade_price: u64,
    pub trade_amount: u64,
    pub trade_type: PerpTradeType,
    pub fee: u64,
    pub new_entry_price: u64,
}

/// Event emitted when a perpetual position is closed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpPositionClosedEvent {
    pub position_id: ObjectID,
    pub market: Address,
    pub owner: Address,
    pub side: PositionSide,
    pub final_size: u64,
    pub entry_price: u64,
    pub close_price: u64,
    pub realized_pnl: i64,
    pub total_fees: u64,
}

/// Extract market_id from transaction events
pub fn extract_perp_market_id_from_events(receipt: &TransactionReceipt) -> Option<ObjectID> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "perp_market_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(market_created_event) = bincode::deserialize::<PerpMarketCreatedEvent>(data) {
                        return Some(market_created_event.market_id);
                    }
                }
            }
        }
    }
    None
}

/// Extract market_address from transaction events
pub fn extract_perp_market_address_from_events(receipt: &TransactionReceipt) -> Option<Address> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "perp_market_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(market_created_event) = bincode::deserialize::<PerpMarketCreatedEvent>(data) {
                        return Some(market_created_event.market_address);
                    }
                }
            }
        }
    }
    None
}

/// Extract order_id from transaction events
pub fn extract_perp_order_id_from_events(receipt: &TransactionReceipt) -> Option<OrderId> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            match action_name.as_str() {
                "perp_order_created" => {
                    if let EventData::Bytes(data) = &event.data {
                        if let Ok(order_created_event) = bincode::deserialize::<PerpOrderCreatedEvent>(data) {
                            return Some(order_created_event.order_id);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Human readable event data
#[derive(Debug, Serialize)]
pub struct HumanReadablePerpEvent {
    pub event_type: String,
    pub sender: Option<String>,
    pub contract: Option<String>,
    pub block_num: u64,
    pub data: serde_json::Value,
}

/// Convert raw perp event data to human readable format
pub fn parse_perp_event_data(event_type: &EventType, data: &EventData) -> Option<serde_json::Value> {
    match (event_type, data) {
        (EventType::Call(action_name), EventData::Bytes(bytes)) => {
            match action_name.as_str() {
                "perp_market_created" => {
                    if let Ok(event) = bincode::deserialize::<PerpMarketCreatedEvent>(bytes) {
                        Some(serde_json::json!({
                            "market_id": event.market_id.to_string(),
                            "market_address": format!("0x{}", hex::encode(event.market_address.as_bytes())),
                            "name": event.name,
                            "base_token": format!("0x{}", hex::encode(event.base_token.as_bytes())),
                            "collateral_token": format!("0x{}", hex::encode(event.collateral_token.as_bytes())),
                            "margin_vault": event.margin_vault.to_string(),
                            "price_index_id": event.price_index_id.to_string(),
                            "oracle_price_feed_id": event.oracle_price_feed_id.to_string(),
                            "min_order_size": format_token_amount(event.min_order_size),
                            "tick_size": format_token_amount(event.tick_size),
                            "maker_fee_bps": event.maker_fee_bps,
                            "taker_fee_bps": event.taker_fee_bps,
                            "allow_market_orders": event.allow_market_orders,
                            "max_leverage": event.max_leverage,
                            "initial_margin_bps": event.initial_margin_bps,
                            "maintenance_margin_bps": event.maintenance_margin_bps,
                            "liquidation_fee_bps": event.liquidation_fee_bps,
                            "max_position_size": format_token_amount(event.max_position_size),
                            "max_price_deviation_bps": event.max_price_deviation_bps,
                            "max_funding_rate_bps": event.max_funding_rate_bps,
                            "creator": format!("0x{}", hex::encode(event.creator.as_bytes()))
                        }))
                    } else { None }
                },
                "perp_market_updated" => {
                    if let Ok(event) = bincode::deserialize::<PerpMarketUpdatedEvent>(bytes) {
                        Some(serde_json::json!({
                            "market_id": event.market_id.to_string(),
                            "min_order_size": event.min_order_size.map(|s| format_token_amount(s)),
                            "tick_size": event.tick_size.map(|t| format_token_amount(t)),
                            "maker_fee_bps": event.maker_fee_bps,
                            "taker_fee_bps": event.taker_fee_bps,
                            "allow_market_orders": event.allow_market_orders,
                            "max_leverage": event.max_leverage,
                            "initial_margin_bps": event.initial_margin_bps,
                            "maintenance_margin_bps": event.maintenance_margin_bps,
                            "liquidation_fee_bps": event.liquidation_fee_bps,
                            "max_position_size": event.max_position_size.map(|s| format_token_amount(s)),
                            "max_price_deviation_bps": event.max_price_deviation_bps,
                            "max_funding_rate_bps": event.max_funding_rate_bps,
                            "updater": format!("0x{}", hex::encode(event.updater.as_bytes()))
                        }))
                    } else { None }
                },
                "perp_order_created" => {
                    if let Ok(event) = bincode::deserialize::<PerpOrderCreatedEvent>(bytes) {
                        let order_type_data = match &event.order_type {
                            PerpOrderEventType::Limit { time_in_force } => {
                                serde_json::json!({
                                    "type": "limit",
                                    "time_in_force": format!("{:?}", time_in_force)
                                })
                            },
                            PerpOrderEventType::Market { slippage } => {
                                serde_json::json!({
                                    "type": "market",
                                    "slippage": format_token_amount(*slippage)
                                })
                            },
                            PerpOrderEventType::Trigger { trigger_price, limit_price, is_market, trigger_type } => {
                                serde_json::json!({
                                    "type": "trigger",
                                    "trigger_price": format_token_amount(*trigger_price),
                                    "limit_price": format_token_amount(*limit_price),
                                    "is_market": is_market,
                                    "trigger_type": format!("{:?}", trigger_type)
                                })
                            }
                        };
                        Some(serde_json::json!({
                            "order_id": event.order_id.to_string(),
                            "market": format!("0x{}", hex::encode(event.market.as_bytes())),
                            "side": format!("{:?}", event.side),
                            "position_side": format!("{:?}", event.position_side),
                            "order_type": order_type_data,
                            "size": format_token_amount(event.size),
                            "filled_size": format_token_amount(event.filled_size),
                            "limit_price": format_token_amount(event.limit_price),
                            "leverage_mode": format!("{:?}", event.leverage_mode),
                            "leverage": event.leverage,
                            "collateral": event.collateral.map(|c| format_token_amount(c)),
                            "reduce_only": event.reduce_only,
                            "trigger": event.trigger.as_ref().map(|t| serde_json::json!({
                                "take_profit": t.take_profit.map(|tp| format_token_amount(tp)),
                                "stop_loss": t.stop_loss.map(|sl| format_token_amount(sl))
                            })),
                            "creator": format!("0x{}", hex::encode(event.creator.as_bytes()))
                        }))
                    } else { None }
                },
                "perp_order_filled" => {
                    if let Ok(event) = bincode::deserialize::<PerpOrderFilledEvent>(bytes) {
                        Some(serde_json::json!({
                            "order_id": event.order_id.to_string(),
                            "market": format!("0x{}", hex::encode(event.market.as_bytes())),
                            "side": format!("{:?}", event.side),
                            "position_side": format!("{:?}", event.position_side),
                            "filled_price": format_token_amount(event.filled_price),
                            "filled_amount": format_token_amount(event.filled_amount),
                            "remaining_amount": format_token_amount(event.remaining_amount),
                            "is_complete": event.is_complete,
                            "fee": format_token_amount(event.fee)
                        }))
                    } else { None }
                },
                "perp_position_open" => {
                    if let Ok(event) = bincode::deserialize::<PerpPositionOpenEvent>(bytes) {
                        Some(serde_json::json!({
                            "position_id": event.position_id.to_string(),
                            "market": format!("0x{}", hex::encode(event.market.as_bytes())),
                            "owner": format!("0x{}", hex::encode(event.owner.as_bytes())),
                            "side": format!("{:?}", event.side),
                            "size": format_token_amount(event.size),
                            "entry_price": format_token_amount(event.entry_price),
                            "leverage_mode": format!("{:?}", event.leverage_mode),
                            "leverage": event.leverage,
                            "collateral": format_token_amount(event.collateral),
                            "initial_margin": format_token_amount(event.initial_margin),
                            "maintenance_margin": format_token_amount(event.maintenance_margin),
                            "entry_funding_rate": event.entry_funding_rate
                        }))
                    } else { None }
                },
                "perp_position_updated" => {
                    if let Ok(event) = bincode::deserialize::<PerpPositionUpdatedEvent>(bytes) {
                        Some(serde_json::json!({
                            "position_id": event.position_id.to_string(),
                            "market": format!("0x{}", hex::encode(event.market.as_bytes())),
                            "owner": format!("0x{}", hex::encode(event.owner.as_bytes())),
                            "old_size": format_token_amount(event.old_size.unsigned_abs()),
                            "new_size": format_token_amount(event.new_size.unsigned_abs()),
                            "trade_price": format_token_amount(event.trade_price),
                            "trade_amount": format_token_amount(event.trade_amount),
                            "trade_type": format!("{:?}", event.trade_type),
                            "fee": format_token_amount(event.fee),
                            "new_entry_price": format_token_amount(event.new_entry_price)
                        }))
                    } else { None }
                },
                "perp_position_closed" => {
                    if let Ok(event) = bincode::deserialize::<PerpPositionClosedEvent>(bytes) {
                        Some(serde_json::json!({
                            "position_id": event.position_id.to_string(),
                            "market": format!("0x{}", hex::encode(event.market.as_bytes())),
                            "owner": format!("0x{}", hex::encode(event.owner.as_bytes())),
                            "side": format!("{:?}", event.side),
                            "final_size": format_token_amount(event.final_size),
                            "entry_price": format_token_amount(event.entry_price),
                            "close_price": format_token_amount(event.close_price),
                            "realized_pnl": format_iamount(event.realized_pnl),
                            "total_fees": format_token_amount(event.total_fees)
                        }))
                    } else { None }
                },

                _ => None
            }
        },
        (EventType::Transfer, EventData::Bytes(bytes)) => {
            if let Ok(event) = bincode::deserialize::<TransferEvent>(bytes) {
                Some(serde_json::json!({
                    "token": format!("0x{}", hex::encode(event.token.as_bytes())),
                    "from": format!("0x{}", hex::encode(event.from.as_bytes())),
                    "to": format!("0x{}", hex::encode(event.to.as_bytes())),
                    "amount": format_token_amount(event.amount),
                    "original_balance_id": event.original_balance_id.to_string(),
                    "to_balance_id": event.to_balance_id.to_string(),
                    "remainder_id": event.remainder_id.map(|id| id.to_string()),
                    "remainder": format_token_amount(event.remainder),
                }))
            } else { None }
        },
        _ => None
    }
}

#[derive(Serialize)]
struct PerpReceiptDisplay<'a> {
    status: String,
    events: &'a [HumanReadablePerpEvent],
}

/// Print perp transaction receipt in a human readable format
pub fn print_perp_receipt_json(receipt: &TransactionReceipt) {
    let human_readable_events: Vec<HumanReadablePerpEvent> = receipt.events.iter().map(|event| {
        HumanReadablePerpEvent {
            event_type: match &event.event_type {
                EventType::Call(name) => name.clone(),
                EventType::Transfer => "Transfer".to_string(),
                _ => "Unknown".to_string()
            },
            sender: event.sender.map(|addr| format!("0x{}", hex::encode(addr.as_bytes()))),
            contract: event.contract.map(|addr| format!("0x{}", hex::encode(addr.as_bytes()))),
            block_num: event.block_num,
            data: parse_perp_event_data(&event.event_type, &event.data)
                .unwrap_or(serde_json::json!(null))
        }
    }).collect();

    let display_receipt = PerpReceiptDisplay{
        status: format!("{:?}", receipt.status),
        events: &human_readable_events
    };

    match serde_json::to_string_pretty(&display_receipt) {
        Ok(json_str) => println!("   {}", json_str),
        Err(e) => println!("   ⚠️  Failed to serialize receipt to JSON: {}", e),
    }
} 