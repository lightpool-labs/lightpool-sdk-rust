// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::module::Module;
use crate::lightpool_types::object::ObjectID;

const TAG_MARKET: u8 = 0x01;
const MARKET_SLOT: u8 = 1;
const LP_SLOT: u8 = 2;
const STAKING_SLOT: u8 = 3;
const ACCOUNT_SLOT: u8 = 4;
const STAKE_SLOT: u8 = 5;
const POSITION_SLOT: u8 = 6;
const QUEUE_SLOT: u8 = 7;

const MAX_MARKET_INDEX: u64 = (1u64 << 48) - 1;

fn payload_from_root_slot(slot: u8) -> [u8; ObjectID::PAYLOAD_LENGTH] {
    let mut payload = [0u8; ObjectID::PAYLOAD_LENGTH];
    payload[0] = 0x00;
    payload[1] = slot;
    payload
}

fn payload_from_user(slot: u8, user: Address) -> [u8; ObjectID::PAYLOAD_LENGTH] {
    let mut payload = [0u8; ObjectID::PAYLOAD_LENGTH];
    payload[0] = 0x00;
    payload[1] = slot;
    let user_bytes = user.as_bytes();
    let copy_len = (ObjectID::PAYLOAD_LENGTH - 2).min(user_bytes.len());
    payload[2..2 + copy_len].copy_from_slice(&user_bytes[..copy_len]);
    payload
}

fn payload_from_index(slot: u8, index: u64) -> [u8; ObjectID::PAYLOAD_LENGTH] {
    let mut payload = [0u8; ObjectID::PAYLOAD_LENGTH];
    payload[0] = 0x00;
    payload[1] = slot;
    payload[2..10].copy_from_slice(&index.to_be_bytes());
    payload
}

pub fn paper_module_contract() -> ContractAddress {
    ContractAddress::new(Module::PAPER, [0u8; 7])
}

pub fn paper_market_contract(index: u64) -> Result<ContractAddress, String> {
    if index == 0 || index > MAX_MARKET_INDEX {
        return Err(format!(
            "paper market contract index must be in 1..={}",
            MAX_MARKET_INDEX
        ));
    }
    let mut rest = [0u8; 7];
    rest[0] = TAG_MARKET;
    let bytes = index.to_be_bytes();
    rest[1..7].copy_from_slice(&bytes[2..8]);
    Ok(ContractAddress::new(Module::PAPER, rest))
}

pub fn paper_market_account(market: ContractAddress) -> Address {
    market.to_address()
}

pub fn paper_market_id(market: ContractAddress) -> ObjectID {
    ObjectID::generate(market, payload_from_root_slot(MARKET_SLOT))
}

pub fn paper_lp_id(market: ContractAddress) -> ObjectID {
    ObjectID::generate(market, payload_from_root_slot(LP_SLOT))
}

pub fn paper_staking_id(market: ContractAddress) -> ObjectID {
    ObjectID::generate(market, payload_from_root_slot(STAKING_SLOT))
}

pub fn paper_account_id(market: ContractAddress, user: Address) -> ObjectID {
    ObjectID::generate(market, payload_from_user(ACCOUNT_SLOT, user))
}

pub fn paper_stake_id(market: ContractAddress, user: Address) -> ObjectID {
    ObjectID::generate(market, payload_from_user(STAKE_SLOT, user))
}

pub fn paper_position_id(market: ContractAddress, index: u64) -> ObjectID {
    ObjectID::generate(market, payload_from_index(POSITION_SLOT, index))
}

pub fn paper_queue_entry_id(market: ContractAddress, index: u64) -> ObjectID {
    ObjectID::generate(market, payload_from_index(QUEUE_SLOT, index))
}
