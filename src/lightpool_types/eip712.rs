// Copyright (c) LightPool Labs
// Author: xiaoyu1998

//! EIP-712 signing domain for LightPool user transactions.

use crate::lightpool_types::crypto::{Digest, Keccak256};

pub const LIGHTPOOL_EIP712_NAME: &str = "LightPool";
pub const LIGHTPOOL_EIP712_VERSION: &str = "1";
pub const LIGHTPOOL_EIP712_CHAIN_ID: u64 = 1337;
pub const LIGHTPOOL_EIP712_VERIFYING_CONTRACT: [u8; 20] = [0u8; 20];

fn keccak256(data: &[u8]) -> [u8; 32] {
    Keccak256::digest(data)
}

fn encode_u256(value: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&value.to_be_bytes());
    out
}

fn encode_address(addr: &[u8; 20]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(addr);
    out
}

fn domain_type_hash() -> [u8; 32] {
    keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    )
}

fn lightpool_tx_type_hash() -> [u8; 32] {
    keccak256(b"LightPoolTx(bytes32 digest)")
}

pub fn lightpool_eip712_domain_separator() -> [u8; 32] {
    let mut encoded = Vec::with_capacity(32 * 5);
    encoded.extend_from_slice(&domain_type_hash());
    encoded.extend_from_slice(&keccak256(LIGHTPOOL_EIP712_NAME.as_bytes()));
    encoded.extend_from_slice(&keccak256(LIGHTPOOL_EIP712_VERSION.as_bytes()));
    encoded.extend_from_slice(&encode_u256(LIGHTPOOL_EIP712_CHAIN_ID));
    encoded.extend_from_slice(&encode_address(&LIGHTPOOL_EIP712_VERIFYING_CONTRACT));
    keccak256(&encoded)
}

pub fn lightpool_tx_struct_hash(tx_digest: &Digest) -> [u8; 32] {
    let mut encoded = Vec::with_capacity(64);
    encoded.extend_from_slice(&lightpool_tx_type_hash());
    encoded.extend_from_slice(&tx_digest.0);
    keccak256(&encoded)
}

pub fn eip712_hash_for_tx_digest(tx_digest: &Digest) -> [u8; 32] {
    let mut encoded = Vec::with_capacity(2 + 32 + 32);
    encoded.extend_from_slice(&[0x19, 0x01]);
    encoded.extend_from_slice(&lightpool_eip712_domain_separator());
    encoded.extend_from_slice(&lightpool_tx_struct_hash(tx_digest));
    keccak256(&encoded)
}
