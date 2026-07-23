// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use serde::{Deserialize, Serialize};

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::event_contract_actions::EventContractState;
use crate::lightpool_types::object::ObjectID;
use crate::token_events::format_token_amount;
use crate::{EventData, EventType, TransactionReceipt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContractCreatedEvent {
    pub market_id: ObjectID,
    pub market_address: ContractAddress,
    pub question: String,
    pub oracle: Address,
    pub collateral_token: ContractAddress,
    pub yes_token: ContractAddress,
    pub no_token: ContractAddress,
    pub yes_spot_market: ContractAddress,
    pub no_spot_market: ContractAddress,
    pub resolution_deadline: u64,
    pub state: EventContractState,
    pub creator: Address,
    pub neg_risk_group_id: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContractMintedEvent {
    pub market_id: ObjectID,
    pub market_address: ContractAddress,
    pub amount: u64,
    pub user: Address,
    pub collateral_token: ContractAddress,
    pub yes_token: ContractAddress,
    pub no_token: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContractBurnedEvent {
    pub market_id: ObjectID,
    pub market_address: ContractAddress,
    pub amount: u64,
    pub user: Address,
    pub collateral_token: ContractAddress,
    pub yes_token: ContractAddress,
    pub no_token: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContractResolvedEvent {
    pub market_id: ObjectID,
    pub market_address: ContractAddress,
    pub outcome: u8,
    pub oracle: Address,
    pub yes_token: ContractAddress,
    pub no_token: ContractAddress,
    pub collateral_token: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContractRedeemedEvent {
    pub market_id: ObjectID,
    pub market_address: ContractAddress,
    pub amount: u64,
    pub user: Address,
    pub winning_token: ContractAddress,
    pub collateral_token: ContractAddress,
    pub yes_token: ContractAddress,
    pub no_token: ContractAddress,
    pub outcome: u8,
}

pub fn extract_event_contract_created_from_events(
    receipt: &TransactionReceipt,
) -> Option<EventContractCreatedEvent> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "event_contract_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(created) = bincode::deserialize::<EventContractCreatedEvent>(data) {
                        return Some(created);
                    }
                }
            }
        }
    }
    None
}

pub fn extract_event_contract_market_address_from_events(
    receipt: &TransactionReceipt,
) -> Option<ContractAddress> {
    extract_event_contract_created_from_events(receipt).map(|event| event.market_address)
}

pub fn parse_event_contract_event_data(event_type: &EventType, data: &EventData) -> Option<serde_json::Value> {
    match (event_type, data) {
        (EventType::Call(action_name), EventData::Bytes(bytes)) => match action_name.as_str() {
            "event_contract_created" => {
                if let Ok(event) = bincode::deserialize::<EventContractCreatedEvent>(bytes) {
                    Some(serde_json::json!({
                        "market_id": event.market_id.to_string(),
                        "market_address": event.market_address.to_string(),
                        "question": event.question,
                        "oracle": event.oracle.to_string(),
                        "collateral_token": event.collateral_token.to_string(),
                        "yes_token": event.yes_token.to_string(),
                        "no_token": event.no_token.to_string(),
                        "yes_spot_market": event.yes_spot_market.to_string(),
                        "no_spot_market": event.no_spot_market.to_string(),
                        "resolution_deadline": event.resolution_deadline,
                        "state": event.state.to_string(),
                        "creator": event.creator.to_string(),
                    }))
                } else {
                    None
                }
            }
            "event_contract_minted" => {
                if let Ok(event) = bincode::deserialize::<EventContractMintedEvent>(bytes) {
                    Some(serde_json::json!({
                        "market_id": event.market_id.to_string(),
                        "market_address": event.market_address.to_string(),
                        "amount": format_token_amount(event.amount),
                        "user": event.user.to_string(),
                        "collateral_token": event.collateral_token.to_string(),
                        "yes_token": event.yes_token.to_string(),
                        "no_token": event.no_token.to_string(),
                    }))
                } else {
                    None
                }
            }
            "event_contract_burned" => {
                if let Ok(event) = bincode::deserialize::<EventContractBurnedEvent>(bytes) {
                    Some(serde_json::json!({
                        "market_id": event.market_id.to_string(),
                        "market_address": event.market_address.to_string(),
                        "amount": format_token_amount(event.amount),
                        "user": event.user.to_string(),
                        "collateral_token": event.collateral_token.to_string(),
                        "yes_token": event.yes_token.to_string(),
                        "no_token": event.no_token.to_string(),
                    }))
                } else {
                    None
                }
            }
            "event_contract_resolved" => {
                if let Ok(event) = bincode::deserialize::<EventContractResolvedEvent>(bytes) {
                    Some(serde_json::json!({
                        "market_id": event.market_id.to_string(),
                        "market_address": event.market_address.to_string(),
                        "outcome": event.outcome,
                        "oracle": event.oracle.to_string(),
                        "yes_token": event.yes_token.to_string(),
                        "no_token": event.no_token.to_string(),
                        "collateral_token": event.collateral_token.to_string(),
                    }))
                } else {
                    None
                }
            }
            "event_contract_redeemed" => {
                if let Ok(event) = bincode::deserialize::<EventContractRedeemedEvent>(bytes) {
                    Some(serde_json::json!({
                        "market_id": event.market_id.to_string(),
                        "market_address": event.market_address.to_string(),
                        "amount": format_token_amount(event.amount),
                        "user": event.user.to_string(),
                        "winning_token": event.winning_token.to_string(),
                        "collateral_token": event.collateral_token.to_string(),
                        "yes_token": event.yes_token.to_string(),
                        "no_token": event.no_token.to_string(),
                        "outcome": event.outcome,
                    }))
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn print_event_contract_receipt_json(receipt: &TransactionReceipt) {
    let events: Vec<serde_json::Value> = receipt
        .events
        .iter()
        .filter_map(|event| {
            let data = parse_event_contract_event_data(&event.event_type, &event.data)?;
            let event_type = match &event.event_type {
                EventType::Call(name) => name.clone(),
                EventType::Transfer => "Transfer".to_string(),
                _ => "Unknown".to_string(),
            };
            Some(serde_json::json!({
                "event_type": event_type,
                "sender": event.sender.map(|s| s.to_string()),
                "contract": event.contract.map(|c| c.to_string()),
                "block_num": event.block_num,
                "data": data,
            }))
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "events": events })).unwrap_or_default()
    );
}
