use serde::{Serialize, Deserialize};
use compact_str::CompactString;
use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::object::ObjectID;
use crate::{TransactionReceipt, EventType, EventData};
use crate::token_events::{
    TokenCreatedEvent,
};

/// Helper to format amounts with 6 decimal places (same as spot)
pub fn format_token_amount(amount: u64) -> String {
    let whole = amount / 1_000_000;
    let fraction = amount % 1_000_000;
    if fraction == 0 {
        format!("{}", whole)
    } else {
        format!("{}.{:06}", whole, fraction)
    }
}

/// Pool created event structure (mirrors core spark PoolCreatedEvent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolCreatedEvent {
    pub pool_address: Address,
    pub pool_id: ObjectID,
    pub quote: Address,
    pub token: Address,
    pub name: CompactString,
    pub symbol: CompactString,
    pub total_supply: u64,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_quote_reserves: u64,
    pub market_cap_limit: u64,
    pub quote_balance_id: ObjectID,
    pub token_balance_id: ObjectID,
    pub creator: Address,
}

/// Buy event structure (mirrors core spark BuyEvent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyEvent {
    pub pool_id: ObjectID,
    pub buyer: Address,
    pub token_amount: u64,
    pub quote_amount: u64,
    pub virtual_token_reserves: u64,
    pub virtual_quote_reserves: u64,
    pub timestamp: u64,
}

/// Sell event structure (mirrors core spark SellEvent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellEvent {
    pub pool_id: ObjectID,
    pub seller: Address,
    pub token_amount: u64,
    pub quote_amount: u64,
    pub virtual_token_reserves: u64,
    pub virtual_quote_reserves: u64,
    pub timestamp: u64,
}

/// Pool completed event structure (mirrors core spark PoolCompletedEvent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolCompletedEvent {
    pub pool_id: ObjectID,
    pub final_virtual_token_reserves: u64,
    pub final_virtual_quote_reserves: u64,
    pub final_real_token_reserves: u64,
    pub final_real_quote_reserves: u64,
    pub final_market_cap: u64,
    pub timestamp: u64,
}

/// Extract pool_id from transaction events
pub fn extract_pool_id_from_events(receipt: &TransactionReceipt) -> Option<ObjectID> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "pool_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(evt) = bincode::deserialize::<PoolCreatedEvent>(data) {
                        return Some(evt.pool_id);
                    }
                }
            }
        }
    }
    None
}

/// Extract token address from transaction events (from pool_created)
pub fn extract_spark_token_address_from_events(receipt: &TransactionReceipt) -> Option<Address> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "pool_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(evt) = bincode::deserialize::<PoolCreatedEvent>(data) {
                        return Some(evt.token);
                    }
                }
            }
        }
    }
    None
}

/// Extract pool address from transaction events (from pool_created)
pub fn extract_pool_address_from_events(receipt: &TransactionReceipt) -> Option<Address> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "pool_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(evt) = bincode::deserialize::<PoolCreatedEvent>(data) {
                        return Some(evt.pool_address);
                    }
                }
            }
        }
    }
    None
}

/// Extract the balance_id for a given recipient address from Transfer events
pub fn extract_spark_balance_id_from_events(receipt: &TransactionReceipt, to: &Address) -> Option<ObjectID> {
    for event in &receipt.events {
        if let EventType::Transfer = &event.event_type {
            if let EventData::Bytes(bytes) = &event.data {
                if let Ok(transfer) = bincode::deserialize::<crate::spot_events::TransferEvent>(bytes) {
                    if &transfer.to == to {
                        return Some(transfer.to_balance_id);
                    }
                }
            }
        }
    }
    None
}

/// Human readable event data
#[derive(Debug, Serialize)]
pub struct HumanReadableSparkEvent {
    pub event_type: String,
    pub sender: Option<String>,
    pub contract: Option<String>,
    pub block_num: u64,
    pub data: serde_json::Value,
}

/// Convert raw spark event data to human readable format
pub fn parse_spark_event_data(event_type: &EventType, data: &EventData) -> Option<serde_json::Value> {
    match (event_type, data) {
        (EventType::Call(action_name), EventData::Bytes(bytes)) => {
            match action_name.as_str() {
                "token_created" => {
                    if let Ok(event) = bincode::deserialize::<TokenCreatedEvent>(bytes) {
                        Some(serde_json::json!({
                            "token_id": event.token_id.to_string(),
                            "token_address": format!("0x{}", hex::encode(event.token_address.as_bytes())),
                            "name": event.name,
                            "symbol": event.symbol,
                            "total_supply": format_token_amount(event.total_supply),
                            "creator": format!("0x{}", hex::encode(event.creator.as_bytes())),
                            "mintable": event.mintable,
                            "to": format!("0x{}", hex::encode(event.to.as_bytes())),
                            "balance_id": event.balance_id.to_string()
                        }))
                    } else { None }
                },
                "pool_created" => {
                    if let Ok(event) = bincode::deserialize::<PoolCreatedEvent>(bytes) {
                        Some(serde_json::json!({
                            "pool_address": format!("0x{}", hex::encode(event.pool_address.as_bytes())),
                            "pool_id": event.pool_id.to_string(),
                            "quote": format!("0x{}", hex::encode(event.quote.as_bytes())),
                            "token": format!("0x{}", hex::encode(event.token.as_bytes())),
                            "name": event.name,
                            "symbol": event.symbol,
                            "total_supply": format_token_amount(event.total_supply),
                            "initial_virtual_token_reserves": format_token_amount(event.initial_virtual_token_reserves),
                            "initial_virtual_quote_reserves": format_token_amount(event.initial_virtual_quote_reserves),
                            "market_cap_limit": format_token_amount(event.market_cap_limit),
                            "quote_balance_id": event.quote_balance_id.to_string(),
                            "token_balance_id": event.token_balance_id.to_string(),
                            "creator": format!("0x{}", hex::encode(event.creator.as_bytes())),
                        }))
                    } else { None }
                }
                "buy" => {
                    if let Ok(event) = bincode::deserialize::<BuyEvent>(bytes) {
                        Some(serde_json::json!({
                            "pool_id": event.pool_id.to_string(),
                            "buyer": format!("0x{}", hex::encode(event.buyer.as_bytes())),
                            "token_amount": format_token_amount(event.token_amount),
                            "quote_amount": format_token_amount(event.quote_amount),
                            "virtual_token_reserves": format_token_amount(event.virtual_token_reserves),
                            "virtual_quote_reserves": format_token_amount(event.virtual_quote_reserves),
                            "timestamp": event.timestamp,
                        }))
                    } else { None }
                }
                "sell" => {
                    if let Ok(event) = bincode::deserialize::<SellEvent>(bytes) {
                        Some(serde_json::json!({
                            "pool_id": event.pool_id.to_string(),
                            "seller": format!("0x{}", hex::encode(event.seller.as_bytes())),
                            "token_amount": format_token_amount(event.token_amount),
                            "quote_amount": format_token_amount(event.quote_amount),
                            "virtual_token_reserves": format_token_amount(event.virtual_token_reserves),
                            "virtual_quote_reserves": format_token_amount(event.virtual_quote_reserves),
                            "timestamp": event.timestamp,
                        }))
                    } else { None }
                }
                "pool_completed" => {
                    if let Ok(event) = bincode::deserialize::<PoolCompletedEvent>(bytes) {
                        Some(serde_json::json!({
                            "pool_id": event.pool_id.to_string(),
                            "final_virtual_token_reserves": format_token_amount(event.final_virtual_token_reserves),
                            "final_virtual_quote_reserves": format_token_amount(event.final_virtual_quote_reserves),
                            "final_real_token_reserves": format_token_amount(event.final_real_token_reserves),
                            "final_real_quote_reserves": format_token_amount(event.final_real_quote_reserves),
                            "final_market_cap": format_token_amount(event.final_market_cap),
                            "timestamp": event.timestamp,
                        }))
                    } else { None }
                }
                _ => None
            }
        }
        (EventType::Transfer, EventData::Bytes(bytes)) => {
            if let Ok(event) = bincode::deserialize::<crate::spot_events::TransferEvent>(bytes) {
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
        }
        _ => None
    }
}

#[derive(Serialize)]
struct ReceiptDisplay<'a> {
    status: String,
    events: &'a [HumanReadableSparkEvent],
}

/// Print spark transaction receipt in a human readable format
pub fn print_spark_receipt_json(receipt: &TransactionReceipt) {
    let human_readable_events: Vec<HumanReadableSparkEvent> = receipt.events.iter().map(|event| {
        HumanReadableSparkEvent {
            event_type: match &event.event_type { EventType::Call(name) => name.clone(), EventType::Transfer => "Transfer".to_string(), _ => "Unknown".to_string() },
            sender: event.sender.map(|addr| format!("0x{}", hex::encode(addr.as_bytes()))),
            contract: event.contract.map(|addr| format!("0x{}", hex::encode(addr.as_bytes()))),
            block_num: event.block_num,
            data: parse_spark_event_data(&event.event_type, &event.data).unwrap_or(serde_json::json!(null)),
        }
    }).collect();

    let display_receipt = ReceiptDisplay{
        status: format!("{:?}", receipt.status),
        events: &human_readable_events
    };

    match serde_json::to_string_pretty(&display_receipt) {
        Ok(json_str) => println!("   {}", json_str),
        Err(e) => println!("   ⚠️  Failed to serialize receipt to JSON: {}", e),
    }
} 