// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::contract::ContractAddress;
use crate::{EventData, EventType, TransactionReceipt};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InboundBridgeCreatedEvent {
    pub bridge: ContractAddress,
    pub foreign_chain_id: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InboundLaneRegisteredEvent {
    pub bridge: ContractAddress,
    pub lane_index: u32,
    pub token: ContractAddress,
    pub foreign_token: [u8; 20],
}

pub fn extract_inbound_bridge_created_from_events(
    receipt: &TransactionReceipt,
) -> Option<InboundBridgeCreatedEvent> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name != "create" {
                continue;
            }
            if let EventData::Bytes(data) = &event.data {
                if let Ok(created) = bincode::deserialize::<InboundBridgeCreatedEvent>(data) {
                    return Some(created);
                }
            }
        }
    }
    None
}

pub fn extract_inbound_lane_registered_from_events(
    receipt: &TransactionReceipt,
) -> Option<InboundLaneRegisteredEvent> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name != "reg_lane" {
                continue;
            }
            if let EventData::Bytes(data) = &event.data {
                if let Ok(registered) = bincode::deserialize::<InboundLaneRegisteredEvent>(data) {
                    return Some(registered);
                }
            }
        }
    }
    None
}
