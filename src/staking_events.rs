use serde::{Deserialize, Serialize};

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::token_events::format_token_amount;
use crate::{EventData, EventType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingConfigInitializedEvent {
    pub lpl_token: ContractAddress,
    pub min_bond: u64,
    pub committee_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondLplEvent {
    pub owner: Address,
    pub amount: u64,
    pub pending_bond: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbondLplEvent {
    pub owner: Address,
    pub amount: u64,
    pub pending_unbond: u64,
}

pub fn parse_staking_event_data(event_type: &EventType, data: &EventData) -> Option<serde_json::Value> {
    match (event_type, data) {
        (EventType::Call(action_name), EventData::Bytes(bytes)) => match action_name.as_str() {
            "init_config" => {
                if let Ok(event) = bincode::deserialize::<StakingConfigInitializedEvent>(bytes) {
                    Some(serde_json::json!({
                        "lpl_token": event.lpl_token.to_string(),
                        "min_bond": format_token_amount(event.min_bond),
                        "committee_size": event.committee_size,
                    }))
                } else {
                    None
                }
            }
            "bond_lpl" => {
                if let Ok(event) = bincode::deserialize::<BondLplEvent>(bytes) {
                    Some(serde_json::json!({
                        "owner": event.owner.to_string(),
                        "amount": format_token_amount(event.amount),
                        "pending_bond": format_token_amount(event.pending_bond),
                    }))
                } else {
                    None
                }
            }
            "unbond_lpl" => {
                if let Ok(event) = bincode::deserialize::<UnbondLplEvent>(bytes) {
                    Some(serde_json::json!({
                        "owner": event.owner.to_string(),
                        "amount": format_token_amount(event.amount),
                        "pending_unbond": format_token_amount(event.pending_unbond),
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
