use serde::{Serialize, Deserialize};
use compact_str::CompactString;
use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::object::ObjectID;
use crate::lightpool_types::token_helpers::{
    balance_object_id, token_object_id, TOKEN_SCALE,
};
use crate::{TransactionReceipt, EventType, EventData};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCreatedEvent {
    pub token_address: ContractAddress,
    pub name: CompactString,
    pub symbol: CompactString,
    pub total_supply: u64,
    pub creator: Address,
    pub mintable: bool,
    pub to: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMintedEvent {
    pub token_address: ContractAddress,
    pub amount: u64,
    pub new_total_supply: u64,
    pub to: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEvent {
    pub token: ContractAddress,
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub remainder: u64,
}

pub fn format_token_amount(amount: u64) -> String {
    let whole = amount / TOKEN_SCALE;
    let fraction = amount % TOKEN_SCALE;
    if fraction == 0 {
        format!("{}", whole)
    } else {
        format!("{}.{:06}", whole, fraction)
    }
}

pub fn extract_token_address_from_events(receipt: &TransactionReceipt) -> Option<ContractAddress> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "token_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(token_created_event) = bincode::deserialize::<TokenCreatedEvent>(data) {
                        return Some(token_created_event.token_address);
                    }
                }
            }
        }
    }
    None
}

pub fn extract_token_id_from_events(receipt: &TransactionReceipt) -> Option<ObjectID> {
    extract_token_address_from_events(receipt)
        .map(token_object_id)
}

pub fn extract_balance_id_from_events(receipt: &TransactionReceipt) -> Option<ObjectID> {
    for event in &receipt.events {
        match &event.event_type {
            EventType::Call(action_name) => {
                if action_name == "token_minted" {
                    if let EventData::Bytes(data) = &event.data {
                        if let Ok(token_minted_event) = bincode::deserialize::<TokenMintedEvent>(data) {
                            return Some(balance_object_id(
                                token_minted_event.token_address,
                                token_minted_event.to,
                            ));
                        }
                    }
                } else if action_name == "token_created" {
                    if let EventData::Bytes(data) = &event.data {
                        if let Ok(token_created_event) = bincode::deserialize::<TokenCreatedEvent>(data) {
                            return Some(balance_object_id(
                                token_created_event.token_address,
                                token_created_event.to,
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}

pub fn extract_transfer_remainder_from_events(receipt: &TransactionReceipt) -> Option<u64> {
    for event in &receipt.events {
        if let EventType::Transfer = &event.event_type {
            if let EventData::Bytes(data) = &event.data {
                if let Ok(transfer_event) = bincode::deserialize::<TransferEvent>(data) {
                    return Some(transfer_event.remainder);
                }
            }
        }
    }
    None
}

#[derive(Debug, Serialize)]
pub struct HumanReadableEvent {
    pub block_num: u64,
    pub event_type: String,
    pub sender: Option<String>,
    pub contract: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Serialize)]
struct ReceiptDisplay<'a> {
    status: String,
    events: &'a [HumanReadableEvent],
}

pub fn parse_event_data(event_type: &EventType, data: &EventData) -> Option<serde_json::Value> {
    match (event_type, data) {
        (EventType::Call(action_name), EventData::Bytes(bytes)) => {
            match action_name.as_str() {
                "token_created" => {
                    if let Ok(event) = bincode::deserialize::<TokenCreatedEvent>(bytes) {
                        let token_id = token_object_id(event.token_address);
                        let balance_id = balance_object_id(event.token_address, event.to);
                        Some(serde_json::json!({
                            "token_address": event.token_address.to_string(),
                            "token_id": token_id.to_string(),
                            "name": event.name,
                            "symbol": event.symbol,
                            "total_supply": format_token_amount(event.total_supply),
                            "creator": event.creator.to_string(),
                            "mintable": event.mintable,
                            "to": event.to.to_string(),
                            "balance_id": balance_id.to_string()
                        }))
                    } else { None }
                },
                "token_minted" => {
                    if let Ok(event) = bincode::deserialize::<TokenMintedEvent>(bytes) {
                        let balance_id = balance_object_id(event.token_address, event.to);
                        Some(serde_json::json!({
                            "token_address": event.token_address.to_string(),
                            "amount": format_token_amount(event.amount),
                            "new_total_supply": format_token_amount(event.new_total_supply),
                            "to": event.to.to_string(),
                            "balance_id": balance_id.to_string()
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

pub fn print_receipt_json(receipt: &TransactionReceipt) {
    let human_readable_events: Vec<HumanReadableEvent> = receipt.events.iter().map(|event| {
        HumanReadableEvent {
            event_type: match &event.event_type {
                EventType::Call(name) => name.clone(),
                EventType::Transfer => "Transfer".to_string(),
                _ => "Unknown".to_string()
            },
            sender: event.sender.map(|addr| addr.to_string()),
            contract: event.contract.map(|addr| addr.to_string()),
            block_num: event.block_num,
            data: parse_event_data(&event.event_type, &event.data)
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
