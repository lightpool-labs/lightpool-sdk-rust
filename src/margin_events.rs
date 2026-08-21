// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use serde::{Deserialize, Serialize};

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::token_events::{format_token_amount, HumanReadableEvent, parse_token_event_data};
use crate::{EventData, EventType, TransactionEvent, TransactionReceipt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolCreatedEvent {
    pub pool: ContractAddress,
    pub creator: Address,
    pub token: ContractAddress,
    pub max_ltv_bps: u64,
    pub maint_bps: u64,
    pub liq_bonus_bps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppliedEvent {
    pub pool: ContractAddress,
    pub lender: Address,
    pub amount: u64,
    pub shares: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyWithdrawnEvent {
    pub pool: ContractAddress,
    pub lender: Address,
    pub shares: u64,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginCreatedEvent {
    pub margin: ContractAddress,
    pub pool: ContractAddress,
    pub owner: Address,
    pub mode: u8,
    pub market: Option<ContractAddress>,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralDepositedEvent {
    pub margin: ContractAddress,
    pub user: Address,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralWithdrawnEvent {
    pub margin: ContractAddress,
    pub user: Address,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowedEvent {
    pub margin: ContractAddress,
    pub amount: u64,
    pub debt: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepaidEvent {
    pub margin: ContractAddress,
    pub amount: u64,
    pub debt: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidatedEvent {
    pub margin: ContractAddress,
    pub liquidator: Address,
    pub repay_amount: u64,
    pub seized_amount: u64,
    pub debt: u64,
}

fn find_call_event<'a>(
    receipt: &'a TransactionReceipt,
    name: &str,
) -> Option<&'a [u8]> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == name {
                if let EventData::Bytes(data) = &event.data {
                    return Some(data.as_slice());
                }
            }
        }
    }
    None
}

pub fn extract_pool_created_from_events(
    receipt: &TransactionReceipt,
) -> Option<PoolCreatedEvent> {
    find_call_event(receipt, "margin_pool_created")
        .and_then(|data| bincode::deserialize(data).ok())
}

pub fn extract_supplied_from_events(receipt: &TransactionReceipt) -> Option<SuppliedEvent> {
    find_call_event(receipt, "margin_supplied")
        .and_then(|data| bincode::deserialize(data).ok())
}

pub fn extract_supply_withdrawn_from_events(
    receipt: &TransactionReceipt,
) -> Option<SupplyWithdrawnEvent> {
    find_call_event(receipt, "margin_supply_withdrawn")
        .and_then(|data| bincode::deserialize(data).ok())
}

pub fn extract_margin_created_from_events(
    receipt: &TransactionReceipt,
) -> Option<MarginCreatedEvent> {
    find_call_event(receipt, "margin_account_created")
        .and_then(|data| bincode::deserialize(data).ok())
}

pub fn extract_collateral_deposited_from_events(
    receipt: &TransactionReceipt,
) -> Option<CollateralDepositedEvent> {
    find_call_event(receipt, "margin_collateral_deposited")
        .and_then(|data| bincode::deserialize(data).ok())
}

pub fn extract_borrowed_from_events(receipt: &TransactionReceipt) -> Option<BorrowedEvent> {
    find_call_event(receipt, "margin_borrowed")
        .and_then(|data| bincode::deserialize(data).ok())
}

pub fn extract_liquidated_from_events(receipt: &TransactionReceipt) -> Option<LiquidatedEvent> {
    find_call_event(receipt, "margin_liquidated")
        .and_then(|data| bincode::deserialize(data).ok())
}

pub fn extract_all_liquidated_from_events(
    receipt: &TransactionReceipt,
) -> Vec<LiquidatedEvent> {
    extract_all_liquidated_from_event_list(&receipt.events)
}

pub fn extract_all_liquidated_from_event_list(
    events: &[TransactionEvent],
) -> Vec<LiquidatedEvent> {
    let mut out = Vec::new();
    for event in events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "margin_liquidated" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(ev) = bincode::deserialize(data) {
                        out.push(ev);
                    }
                }
            }
        }
    }
    out
}

pub fn parse_margin_event_data(event_type: &EventType, data: &EventData) -> Option<serde_json::Value> {
    match (event_type, data) {
        (EventType::Call(action_name), EventData::Bytes(bytes)) => match action_name.as_str() {
            "margin_pool_created" => {
                let event: PoolCreatedEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "pool": event.pool.to_string(),
                    "creator": event.creator.to_string(),
                    "token": event.token.to_string(),
                    "max_ltv_bps": event.max_ltv_bps,
                    "maint_bps": event.maint_bps,
                    "liq_bonus_bps": event.liq_bonus_bps,
                }))
            }
            "margin_supplied" => {
                let event: SuppliedEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "pool": event.pool.to_string(),
                    "lender": event.lender.to_string(),
                    "amount": format_token_amount(event.amount),
                    "shares": format_token_amount(event.shares),
                }))
            }
            "margin_supply_withdrawn" => {
                let event: SupplyWithdrawnEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "pool": event.pool.to_string(),
                    "lender": event.lender.to_string(),
                    "shares": format_token_amount(event.shares),
                    "amount": format_token_amount(event.amount),
                }))
            }
            "margin_account_created" => {
                let event: MarginCreatedEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "margin": event.margin.to_string(),
                    "pool": event.pool.to_string(),
                    "owner": event.owner.to_string(),
                    "mode": event.mode,
                    "market": event.market.map(|m| m.to_string()),
                    "amount": format_token_amount(event.amount),
                }))
            }
            "margin_collateral_deposited" => {
                let event: CollateralDepositedEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "margin": event.margin.to_string(),
                    "user": event.user.to_string(),
                    "amount": format_token_amount(event.amount),
                }))
            }
            "margin_collateral_withdrawn" => {
                let event: CollateralWithdrawnEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "margin": event.margin.to_string(),
                    "user": event.user.to_string(),
                    "amount": format_token_amount(event.amount),
                }))
            }
            "margin_borrowed" => {
                let event: BorrowedEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "margin": event.margin.to_string(),
                    "amount": format_token_amount(event.amount),
                    "debt": format_token_amount(event.debt),
                }))
            }
            "margin_repaid" => {
                let event: RepaidEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "margin": event.margin.to_string(),
                    "amount": format_token_amount(event.amount),
                    "debt": format_token_amount(event.debt),
                }))
            }
            "margin_liquidated" => {
                let event: LiquidatedEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "margin": event.margin.to_string(),
                    "liquidator": event.liquidator.to_string(),
                    "repay_amount": format_token_amount(event.repay_amount),
                    "seized_amount": format_token_amount(event.seized_amount),
                    "debt": format_token_amount(event.debt),
                }))
            }
            _ => None,
        },
        _ => None,
    }
}

#[derive(Serialize)]
struct ReceiptDisplay<'a> {
    status: String,
    events: &'a [HumanReadableEvent],
}

/// Pretty-print margin receipts like token Transfer (`print_receipt_json`).
pub fn print_margin_receipt_json(receipt: &TransactionReceipt) {
    let human_readable_events: Vec<HumanReadableEvent> = receipt
        .events
        .iter()
        .map(|event| HumanReadableEvent {
            event_type: match &event.event_type {
                EventType::Call(name) => name.clone(),
                EventType::Transfer => "Transfer".to_string(),
                other => other.to_string(),
            },
            contract: event.contract.map(|addr| addr.to_string()),
            data: parse_margin_event_data(&event.event_type, &event.data)
                .or_else(|| parse_token_event_data(&event.event_type, &event.data))
                .unwrap_or(serde_json::json!(null)),
        })
        .collect();

    let display_receipt = ReceiptDisplay {
        status: format!("{:?}", receipt.status),
        events: &human_readable_events,
    };

    match serde_json::to_string_pretty(&display_receipt) {
        Ok(json_str) => println!("   {}", json_str),
        Err(e) => println!("   Failed to serialize receipt to JSON: {}", e),
    }
}
