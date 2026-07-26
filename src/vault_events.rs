// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use serde::{Deserialize, Serialize};

use compact_str::CompactString;
use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::token_events::format_token_amount;
use crate::{EventData, EventType, TransactionReceipt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultCreatedEvent {
    pub vault: ContractAddress,
    pub name: CompactString,
    pub manager: Address,
    pub quote_token: ContractAddress,
    pub share_token: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDepositedEvent {
    pub vault: ContractAddress,
    pub user: Address,
    pub amount: u64,
    pub shares: u64,
    pub equity: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultWithdrawnEvent {
    pub vault: ContractAddress,
    pub user: Address,
    pub amount: u64,
    pub shares: u64,
    pub equity: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultManagerUpdatedEvent {
    pub vault: ContractAddress,
    pub old_manager: Address,
    pub new_manager: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDepositPermissionUpdatedEvent {
    pub vault: ContractAddress,
    pub allow_deposit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultPerformanceFeeAccruedEvent {
    pub vault: ContractAddress,
    pub profit: u64,
    pub fee_quote: u64,
    pub shares_minted: u64,
    pub high_water_mark: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultClosedEvent {
    pub vault: ContractAddress,
}

pub fn extract_vault_created_from_events(
    receipt: &TransactionReceipt,
) -> Option<VaultCreatedEvent> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "vault_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(created) = bincode::deserialize::<VaultCreatedEvent>(data) {
                        return Some(created);
                    }
                }
            }
        }
    }
    None
}

pub fn extract_vault_address_from_events(
    receipt: &TransactionReceipt,
) -> Option<ContractAddress> {
    extract_vault_created_from_events(receipt).map(|event| event.vault)
}

pub fn parse_vault_event_data(event_type: &EventType, data: &EventData) -> Option<serde_json::Value> {
    match (event_type, data) {
        (EventType::Call(action_name), EventData::Bytes(bytes)) => match action_name.as_str() {
            "vault_created" => {
                if let Ok(event) = bincode::deserialize::<VaultCreatedEvent>(bytes) {
                    Some(serde_json::json!({
                        "vault": event.vault.to_string(),
                        "name": event.name.to_string(),
                        "manager": event.manager.to_string(),
                        "quote_token": event.quote_token.to_string(),
                        "share_token": event.share_token.to_string(),
                    }))
                } else {
                    None
                }
            }
            "vault_deposited" => {
                if let Ok(event) = bincode::deserialize::<VaultDepositedEvent>(bytes) {
                    Some(serde_json::json!({
                        "vault": event.vault.to_string(),
                        "user": event.user.to_string(),
                        "amount": format_token_amount(event.amount),
                        "shares": format_token_amount(event.shares),
                        "equity": format_token_amount(event.equity),
                    }))
                } else {
                    None
                }
            }
            "vault_withdrawn" => {
                if let Ok(event) = bincode::deserialize::<VaultWithdrawnEvent>(bytes) {
                    Some(serde_json::json!({
                        "vault": event.vault.to_string(),
                        "user": event.user.to_string(),
                        "amount": format_token_amount(event.amount),
                        "shares": format_token_amount(event.shares),
                        "equity": format_token_amount(event.equity),
                    }))
                } else {
                    None
                }
            }
            "vault_manager_updated" => {
                if let Ok(event) = bincode::deserialize::<VaultManagerUpdatedEvent>(bytes) {
                    Some(serde_json::json!({
                        "vault": event.vault.to_string(),
                        "old_manager": event.old_manager.to_string(),
                        "new_manager": event.new_manager.to_string(),
                    }))
                } else {
                    None
                }
            }
            "vault_deposit_permission_updated" => {
                if let Ok(event) =
                    bincode::deserialize::<VaultDepositPermissionUpdatedEvent>(bytes)
                {
                    Some(serde_json::json!({
                        "vault": event.vault.to_string(),
                        "allow_deposit": event.allow_deposit,
                    }))
                } else {
                    None
                }
            }
            "vault_performance_fee_accrued" => {
                if let Ok(event) = bincode::deserialize::<VaultPerformanceFeeAccruedEvent>(bytes) {
                    Some(serde_json::json!({
                        "vault": event.vault.to_string(),
                        "profit": format_token_amount(event.profit),
                        "fee_quote": format_token_amount(event.fee_quote),
                        "shares_minted": format_token_amount(event.shares_minted),
                        "high_water_mark": format_token_amount(event.high_water_mark),
                    }))
                } else {
                    None
                }
            }
            "vault_closed" => {
                if let Ok(event) = bincode::deserialize::<VaultClosedEvent>(bytes) {
                    Some(serde_json::json!({
                        "vault": event.vault.to_string(),
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

pub fn print_vault_receipt_json(receipt: &TransactionReceipt) {
    let mut events = Vec::new();
    for event in &receipt.events {
        if let Some(parsed) = parse_vault_event_data(&event.event_type, &event.data) {
            events.push(parsed);
        }
    }
    let payload = serde_json::json!({
        "status": format!("{:?}", receipt.status),
        "events": events,
    });
    println!("{}", serde_json::to_string_pretty(&payload).unwrap_or_default());
}
