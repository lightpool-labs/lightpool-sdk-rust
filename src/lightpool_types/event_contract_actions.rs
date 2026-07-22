use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventContractState {
    Created,
    Active,
    Closed,
    Resolved,
}

impl std::fmt::Display for EventContractState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventContractState::Created => write!(f, "Created"),
            EventContractState::Active => write!(f, "Active"),
            EventContractState::Closed => write!(f, "Closed"),
            EventContractState::Resolved => write!(f, "Resolved"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventContractParams {
    pub question: String,
    pub oracle: Address,
    pub collateral_token: ContractAddress,
    pub resolution_deadline: u64,
    pub tick_size: u64,
    pub min_order_size: u64,
    pub maker_fee_bps: u16,
    pub taker_fee_bps: u16,
    pub allow_market_orders: bool,
    pub neg_risk_group_id: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEventContractParams {
    pub amount: u64,
    pub collateral_token: ContractAddress,
    pub yes_token: ContractAddress,
    pub no_token: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnEventContractParams {
    pub amount: u64,
    pub collateral_token: ContractAddress,
    pub yes_token: ContractAddress,
    pub no_token: ContractAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveEventContractParams {
    pub outcome: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemEventContractParams {
    pub collateral_token: ContractAddress,
    pub yes_token: ContractAddress,
    pub no_token: ContractAddress,
}
