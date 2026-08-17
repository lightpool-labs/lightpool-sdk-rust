// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use serde::{Deserialize, Serialize};

/// Block-end clearinghouse outcomes (not module TransactionEvents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClearingHouseEvent {
    Liquidated {
        margin: ContractAddress,
        liquidator: Address,
        repay_amount: u64,
        seized_amount: u64,
        debt: u64,
    },
}
