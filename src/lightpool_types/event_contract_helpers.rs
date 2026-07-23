// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::module::Module;

pub fn event_contract_module_contract() -> ContractAddress {
    ContractAddress::new(Module::EVENT_CONTRACT, [0u8; 7])
}
