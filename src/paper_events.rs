// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::effects::{EventData, EventType, TransactionReceipt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMarketCreatedEvent {
    pub market: ContractAddress,
    pub admin: Address,
    pub quote_token: ContractAddress,
    pub paper_token: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperInstrumentAddedEvent {
    pub market: ContractAddress,
    pub instrument: u32,
    pub name: CompactString,
    pub price_market: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperOpenedEvent {
    pub market: ContractAddress,
    pub position: u64,
    pub owner: Address,
    pub instrument: u32,
    pub is_long: bool,
    pub margin: u64,
    pub notional: u64,
    pub entry_price: u64,
    pub bust_price: u64,
    pub funded_from_queued: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperClosedEvent {
    pub market: ContractAddress,
    pub position: u64,
    pub owner: Address,
    pub exit_price: u64,
    pub pnl: i64,
    pub fee: u64,
    pub payout: u64,
    pub queued: u64,
    pub minted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperLiquidatedEvent {
    pub market: ContractAddress,
    pub position: u64,
    pub owner: Address,
    pub liquidator: Address,
    pub exit_price: u64,
    pub margin: u64,
    pub minted: u64,
}

fn find_call_event<'a>(
    receipt: &'a TransactionReceipt,
    action_name: &str,
) -> Option<&'a [u8]> {
    for event in &receipt.events {
        let EventType::Call(name) = &event.event_type else {
            continue;
        };
        if name != action_name {
            continue;
        }
        if let EventData::Bytes(data) = &event.data {
            return Some(data.as_slice());
        }
    }
    None
}

pub fn extract_paper_market_created(
    receipt: &TransactionReceipt,
) -> Option<PaperMarketCreatedEvent> {
    let data = find_call_event(receipt, "paper_market_created")?;
    bincode::deserialize(data).ok()
}

pub fn extract_paper_instrument_added(
    receipt: &TransactionReceipt,
) -> Option<PaperInstrumentAddedEvent> {
    let data = find_call_event(receipt, "paper_instrument_added")?;
    bincode::deserialize(data).ok()
}

pub fn extract_paper_opened(receipt: &TransactionReceipt) -> Option<PaperOpenedEvent> {
    let data = find_call_event(receipt, "paper_opened")?;
    bincode::deserialize(data).ok()
}

pub fn extract_paper_closed(receipt: &TransactionReceipt) -> Option<PaperClosedEvent> {
    let data = find_call_event(receipt, "paper_closed")?;
    bincode::deserialize(data).ok()
}

pub fn extract_paper_liquidated(receipt: &TransactionReceipt) -> Option<PaperLiquidatedEvent> {
    let data = find_call_event(receipt, "paper_liquidated")?;
    bincode::deserialize(data).ok()
}
