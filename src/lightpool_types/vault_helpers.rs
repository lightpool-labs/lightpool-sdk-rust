// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::module::Module;
use crate::lightpool_types::object::ObjectID;

const PORTFOLIO_SLOT: u8 = 2;

fn payload_from_root_slot(slot: u8) -> [u8; ObjectID::PAYLOAD_LENGTH] {
    let mut payload = [0u8; ObjectID::PAYLOAD_LENGTH];
    payload[0] = 0x00;
    payload[1] = slot;
    payload
}

pub fn vault_module_contract() -> ContractAddress {
    ContractAddress::new(Module::VAULT, [0u8; 7])
}

pub fn vault_contract(index: u64) -> Result<ContractAddress, String> {
    const MAX_VAULT_INDEX: u64 = (1u64 << 56) - 1;
    if index == 0 || index > MAX_VAULT_INDEX {
        return Err(format!(
            "vault contract index must be in 1..={}",
            MAX_VAULT_INDEX
        ));
    }
    let mut rest = [0u8; 7];
    rest.copy_from_slice(&index.to_be_bytes()[1..]);
    Ok(ContractAddress::new(Module::VAULT, rest))
}

pub fn vault_account(vault: ContractAddress) -> Address {
    vault.to_address()
}

pub fn vault_portfolio_id(vault: ContractAddress) -> ObjectID {
    ObjectID::generate(vault, payload_from_root_slot(PORTFOLIO_SLOT))
}
