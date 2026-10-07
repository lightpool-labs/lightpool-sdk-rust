// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use serde::{Deserialize, Serialize};

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::{EventData, EventType, TransactionEvent, TransactionReceipt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmPoolCreatedEvent {
    pub pool: ContractAddress,
    pub token0: ContractAddress,
    pub token1: ContractAddress,
    pub fee: u32,
    pub tick_spacing: i32,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub creator: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmLiquidityChangedEvent {
    pub pool: ContractAddress,
    pub owner: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_delta: i128,
    pub amount0: u64,
    pub amount1: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmSwappedEvent {
    pub pool: ContractAddress,
    pub sender: Address,
    pub zero_for_one: bool,
    pub amount0: u64,
    pub amount1: u64,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
}

pub fn extract_amm_pool_from_events(receipt: &TransactionReceipt) -> Option<ContractAddress> {
    extract_amm_pool_from_event_list(&receipt.events)
}

pub fn extract_amm_pool_from_event_list(events: &[TransactionEvent]) -> Option<ContractAddress> {
    for event in events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "pool_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(created) = bincode::deserialize::<AmmPoolCreatedEvent>(data) {
                        return Some(created.pool);
                    }
                }
            }
        }
    }
    None
}

pub fn parse_amm_event_data(event_type: &EventType, data: &EventData) -> Option<serde_json::Value> {
    match (event_type, data) {
        (EventType::Call(action_name), EventData::Bytes(bytes)) => match action_name.as_str() {
            "pool_created" => {
                let event: AmmPoolCreatedEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "pool": event.pool.to_string(),
                    "token0": event.token0.to_string(),
                    "token1": event.token1.to_string(),
                    "fee": event.fee,
                    "tick_spacing": event.tick_spacing,
                    "sqrt_price_x96": event.sqrt_price_x96.to_string(),
                    "tick": event.tick,
                    "creator": event.creator.to_string(),
                }))
            }
            "liquidity_changed" => {
                let event: AmmLiquidityChangedEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "pool": event.pool.to_string(),
                    "owner": event.owner.to_string(),
                    "tick_lower": event.tick_lower,
                    "tick_upper": event.tick_upper,
                    "liquidity_delta": event.liquidity_delta.to_string(),
                    "amount0": event.amount0,
                    "amount1": event.amount1,
                }))
            }
            "swapped" => {
                let event: AmmSwappedEvent = bincode::deserialize(bytes).ok()?;
                Some(serde_json::json!({
                    "pool": event.pool.to_string(),
                    "sender": event.sender.to_string(),
                    "zero_for_one": event.zero_for_one,
                    "amount0": event.amount0,
                    "amount1": event.amount1,
                    "sqrt_price_x96": event.sqrt_price_x96.to_string(),
                    "tick": event.tick,
                    "liquidity": event.liquidity.to_string(),
                }))
            }
            _ => None,
        },
        _ => None,
    }
}
