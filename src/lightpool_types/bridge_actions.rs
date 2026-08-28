// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::crypto::{PublicKey, Signature};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeAuthority {
    pub consensus_pubkey: PublicKey,
    pub stake: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeVote {
    pub validator: PublicKey,
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeDepositMessage {
    pub lane_index: u32,
    pub message_id: u64,
    pub source_chain_id: u64,
    pub token: ContractAddress,
    pub amount: u64,
    pub sender_foreign: [u8; 20],
    pub recipient: Address,
    pub source_tx_hash: [u8; 32],
    pub source_block: u64,
    pub epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterInboundLaneParams {
    pub foreign_token: [u8; 20],
    pub name: CompactString,
    pub symbol: CompactString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInboundBridgeParams {
    pub foreign_chain_id: u64,
    #[serde(default)]
    pub epoch: u64,
    #[serde(default)]
    pub authorities: Option<Vec<BridgeAuthority>>,
    #[serde(default)]
    pub first_lane: Option<RegisterInboundLaneParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmDepositParams {
    pub message: BridgeDepositMessage,
    pub votes: Vec<BridgeVote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeWithdrawParams {
    pub token: ContractAddress,
    pub amount: u64,
    pub foreign_recipient: [u8; 20],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterOutboundLaneParams {
    pub token: ContractAddress,
    pub foreign_token: [u8; 20],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOutboundBridgeParams {
    pub foreign_chain_id: u64,
    pub epoch: u64,
    pub authorities: Vec<BridgeAuthority>,
    #[serde(default)]
    pub first_lane: Option<RegisterOutboundLaneParams>,
}
