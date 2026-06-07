use serde::{Serialize, Deserialize};
use compact_str::CompactString;
use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::object::ObjectID;
use crate::lightpool_types::spot_actions::{MarketState, OrderSide, TimeInForce, TriggerType};
use crate::lightpool_types::OrderId;
use crate::token_events::{TransferEvent, format_token_amount};
use crate::{TransactionReceipt, EventType, EventData};

/// Market created event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketCreatedEvent {
    pub market_id: ObjectID,
    pub market_address: ContractAddress,
    pub bids_id: ObjectID,
    pub asks_id: ObjectID,
    pub name: CompactString,
    pub base_token: Address,
    pub quote_token: Address,
    pub min_order_size: u64,
    pub tick_size: u64,
    pub maker_fee_bps: u16,
    pub taker_fee_bps: u16,
    pub allow_market_orders: bool,
    pub state: MarketState,
    pub creator: Address,
}

/// Market updated event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUpdatedEvent {
    pub market_id: ObjectID,
    pub min_order_size: Option<u64>,
    pub maker_fee_bps: Option<u16>,
    pub taker_fee_bps: Option<u16>,
    pub allow_market_orders: Option<bool>,
    pub state: Option<MarketState>,
    pub updater: Address,
}

/// Order event type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderEventType {
    Limit {
        price: u64,
        tif: TimeInForce,
    },
    Market {
        slippage: u64,
    },
    Trigger {
        trigger_price: u64,
        limit_price: u64,
        is_market: bool,
        trigger_type: TriggerType,
    },
}

/// Unified order created event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreatedEvent {
    pub order_id: OrderId,
    pub side: OrderSide,
    pub amount: u64,
    pub creator: Address,
    pub market: Address,
    pub order_type: OrderEventType,
}

/// Market order executed event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketOrderExecutedEvent {
    pub order_id: OrderId,
    pub side: OrderSide,
    pub amount: u64,
    pub filled_amount: u64,
    pub avg_filled_price: Option<u64>,
    pub creator: Address,
}

/// Order filled event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFilledEvent {
    pub order_id: OrderId,
    pub side: OrderSide,
    pub price: u64,
    pub fill_amount: u64,
    pub remaining_amount: u64,
    pub is_fully_filled: bool,
    pub market: Address,
}

/// Order cancelled event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCancelledEvent {
    pub order_id: OrderId,
    pub side: OrderSide,
    pub price: u64,
    pub cancelled_amount: u64,
    pub reason: String,
}

/// Trigger order activated event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerOrderActivatedEvent {
    pub order_id: OrderId,
    pub trigger_price: u64,
    pub activation_price: u64,
    pub trigger_type: TriggerType,
}

/// Extract market_id from transaction events
pub fn extract_market_id_from_events(receipt: &TransactionReceipt) -> Option<ObjectID> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "market_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(market_created_event) = bincode::deserialize::<MarketCreatedEvent>(data) {
                        return Some(market_created_event.market_id);
                    }
                }
            }
        }
    }
    None
}

/// Extract market contract address from transaction events
pub fn extract_market_address_from_events(receipt: &TransactionReceipt) -> Option<ContractAddress> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "market_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(market_created_event) = bincode::deserialize::<MarketCreatedEvent>(data) {
                        return Some(market_created_event.market_address);
                    }
                }
            }
        }
    }
    None
}

/// Extract order_id from transaction events
pub fn extract_order_id_from_events(receipt: &TransactionReceipt) -> Option<OrderId> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            match action_name.as_str() {
                "order_created" => {
                    if let EventData::Bytes(data) = &event.data {
                        if let Ok(order_created_event) = bincode::deserialize::<OrderCreatedEvent>(data) {
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
pub struct HumanReadableSpotEvent {
    pub event_type: String,
    pub sender: Option<String>,
    pub contract: Option<String>,
    pub block_num: u64,
    pub data: serde_json::Value,
}

/// Convert raw spot event data to human readable format
pub fn parse_spot_event_data(event_type: &EventType, data: &EventData) -> Option<serde_json::Value> {
    match (event_type, data) {
        (EventType::Call(action_name), EventData::Bytes(bytes)) => {
            match action_name.as_str() {
                "market_created" => {
                    if let Ok(event) = bincode::deserialize::<MarketCreatedEvent>(bytes) {
                        Some(serde_json::json!({
                            "market_id": event.market_id.to_string(),
                            "market_address": event.market_address.to_string(),
                            "bids_id": event.bids_id.to_string(),
                            "asks_id": event.asks_id.to_string(),
                            "name": event.name,
                            "base_token": event.base_token.to_string(),
                            "quote_token": event.quote_token.to_string(),
                            "min_order_size": format_token_amount(event.min_order_size),
                            "tick_size": format_token_amount(event.tick_size),
                            "maker_fee_bps": event.maker_fee_bps,
                            "taker_fee_bps": event.taker_fee_bps,
                            "allow_market_orders": event.allow_market_orders,
                            "state": format!("{}", event.state),
                            "creator": event.creator.to_string()
                        }))
                    } else { None }
                },
                "market_updated" => {
                    if let Ok(event) = bincode::deserialize::<MarketUpdatedEvent>(bytes) {
                        Some(serde_json::json!({
                            "market_id": event.market_id.to_string(),
                            "min_order_size": event.min_order_size.map(format_token_amount),
                            "maker_fee_bps": event.maker_fee_bps,
                            "taker_fee_bps": event.taker_fee_bps,
                            "allow_market_orders": event.allow_market_orders,
                            "state": event.state.as_ref().map(|s| format!("{}", s)),
                            "updater": event.updater.to_string()
                        }))
                    } else { None }
                },
                "order_created" => {
                    if let Ok(event) = bincode::deserialize::<OrderCreatedEvent>(bytes) {
                        let order_type_data = match &event.order_type {
                            OrderEventType::Limit { price, tif } => {
                                serde_json::json!({
                                    "type": "limit",
                                    "price": format_token_amount(*price),
                                    "tif": format!("{:?}", tif)
                                })
                            },
                            OrderEventType::Market { slippage } => {
                                serde_json::json!({
                                    "type": "market",
                                    "slippage": format_token_amount(*slippage)
                                })
                            },
                            OrderEventType::Trigger { trigger_price, limit_price, is_market, trigger_type } => {
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
                            "order_id": event.order_id,
                            "side": format!("{:?}", event.side),
                            "amount": format_token_amount(event.amount),
                            "creator": event.creator.to_string(),
                            "market": event.market.to_string(),
                            "order_type": order_type_data
                        }))
                    } else { None }
                },
                "market_order_executed" => {
                    if let Ok(event) = bincode::deserialize::<MarketOrderExecutedEvent>(bytes) {
                        Some(serde_json::json!({
                            "order_id": event.order_id,
                            "side": format!("{:?}", event.side),
                            "amount": format_token_amount(event.amount),
                            "filled_amount": format_token_amount(event.filled_amount),
                            "avg_filled_price": event.avg_filled_price.map(format_token_amount),
                            "creator": event.creator.to_string()
                        }))
                    } else { None }
                },
                "order_filled" => {
                    if let Ok(event) = bincode::deserialize::<OrderFilledEvent>(bytes) {
                        Some(serde_json::json!({
                            "order_id": event.order_id,
                            "side": format!("{:?}", event.side),
                            "price": format_token_amount(event.price),
                            "fill_amount": format_token_amount(event.fill_amount),
                            "remaining_amount": format_token_amount(event.remaining_amount),
                            "is_fully_filled": event.is_fully_filled,
                            "market": event.market.to_string()
                        }))
                    } else { None }
                },
                "order_cancelled" => {
                    if let Ok(event) = bincode::deserialize::<OrderCancelledEvent>(bytes) {
                        Some(serde_json::json!({
                            "order_id": event.order_id,
                            "side": format!("{:?}", event.side),
                            "price": format_token_amount(event.price),
                            "cancelled_amount": format_token_amount(event.cancelled_amount),
                            "reason": event.reason
                        }))
                    } else { None }
                },
                "trigger_order_activated" => {
                    if let Ok(event) = bincode::deserialize::<TriggerOrderActivatedEvent>(bytes) {
                        Some(serde_json::json!({
                            "order_id": event.order_id,
                            "trigger_price": format_token_amount(event.trigger_price),
                            "activation_price": format_token_amount(event.activation_price),
                            "trigger_type": format!("{:?}", event.trigger_type)
                        }))
                    } else { None }
                },
                _ => None
            }
        },
        (EventType::Transfer, EventData::Bytes(bytes)) => {
            if let Ok(event) = bincode::deserialize::<TransferEvent>(bytes) {
                Some(serde_json::json!({
                    "token": event.token.to_string(),
                    "from": event.from.to_string(),
                    "to": event.to.to_string(),
                    "amount": format_token_amount(event.amount),
                    "remainder": format_token_amount(event.remainder),
                }))
            } else { None }
        },
        _ => None
    }
}

#[derive(Serialize)]
struct ReceiptDisplay<'a> {
    status: String,
    events: &'a [HumanReadableSpotEvent],
}

/// Print spot transaction receipt in a human readable format
pub fn print_spot_receipt_json(receipt: &TransactionReceipt) {
    let human_readable_events: Vec<HumanReadableSpotEvent> = receipt.events.iter().map(|event| {
        HumanReadableSpotEvent {
            event_type: match &event.event_type {
                EventType::Call(name) => name.clone(),
                EventType::Transfer => "Transfer".to_string(),
                _ => "Unknown".to_string()
            },
            sender: event.sender.map(|addr| addr.to_string()),
            contract: event.contract.map(|addr| addr.to_string()),
            block_num: event.block_num,
            data: parse_spot_event_data(&event.event_type, &event.data)
                .unwrap_or(serde_json::json!(null))
        }
    }).collect();

    let display_receipt = ReceiptDisplay{
        status: format!("{:?}", receipt.status),
        events: &human_readable_events
    };

    match serde_json::to_string_pretty(&display_receipt) {
        Ok(json_str) => println!("   {}", json_str),
        Err(e) => println!("   Failed to serialize receipt to JSON: {}", e),
    }
}
