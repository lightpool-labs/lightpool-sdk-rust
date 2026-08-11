// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::module::Module;

pub const TAG_POOL: u8 = 0x01;
pub const TAG_ACCOUNT: u8 = 0x02;
pub const MAX_MARGIN_INDEX: u64 = (1u64 << 48) - 1;

pub fn margin_module_contract() -> ContractAddress {
    ContractAddress::new(Module::MARGIN, [0u8; 7])
}

fn contract_from_tag_index(tag: u8, index: u64) -> Result<ContractAddress, String> {
    if index == 0 || index > MAX_MARGIN_INDEX {
        return Err(format!(
            "margin contract index must be in 1..={}",
            MAX_MARGIN_INDEX
        ));
    }
    let mut rest = [0u8; 7];
    rest[0] = tag;
    let bytes = index.to_be_bytes();
    rest[1..7].copy_from_slice(&bytes[2..8]);
    Ok(ContractAddress::new(Module::MARGIN, rest))
}

pub fn pool_contract(index: u64) -> Result<ContractAddress, String> {
    contract_from_tag_index(TAG_POOL, index)
}

pub fn margin_account_contract(index: u64) -> Result<ContractAddress, String> {
    contract_from_tag_index(TAG_ACCOUNT, index)
}

pub fn pool_account(pool: ContractAddress) -> Address {
    pool.to_address()
}

pub fn margin_trading_account(margin: ContractAddress) -> Address {
    margin.to_address()
}

pub fn is_pool_contract(contract: ContractAddress) -> bool {
    contract.module() == Module::MARGIN && contract.rest()[0] == TAG_POOL
}

pub fn is_margin_account_contract(contract: ContractAddress) -> bool {
    contract.module() == Module::MARGIN && contract.rest()[0] == TAG_ACCOUNT
}
