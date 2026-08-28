// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::module::Module;

pub fn bridge_module_contract() -> ContractAddress {
    ContractAddress::new(Module::BRIDGE, [0u8; 7])
}

pub fn default_inbound_bridge_instance() -> ContractAddress {
    let mut rest = [0u8; 7];
    rest.copy_from_slice(&1u64.to_be_bytes()[1..]);
    ContractAddress::new(Module::BRIDGE, rest)
}
