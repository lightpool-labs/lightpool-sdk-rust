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
    pub message_id: u64,
    pub source_chain_id: u64,
    pub token: ContractAddress,
    pub amount: u64,
    pub sender_evm: [u8; 20],
    pub recipient: Address,
    pub source_tx_hash: [u8; 32],
    pub source_block: u64,
    pub epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitBridgeConfigParams {
    pub evm_chain_id: u64,
    /// ERC20 token address on the source EVM chain (e.g. USDT).
    pub evm_token: [u8; 20],
    pub name: CompactString,
    pub symbol: CompactString,
    pub epoch: u64,
    pub authorities: Vec<BridgeAuthority>,
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
    pub evm_recipient: [u8; 20],
}
