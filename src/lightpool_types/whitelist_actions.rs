// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::address_type::Address;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWhitelistParams {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistAddParams {
    pub user: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistRemoveParams {
    pub user: Address,
}

impl fmt::Display for CreateWhitelistParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CreateWhitelist")
    }
}

impl fmt::Display for WhitelistAddParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WhitelistAdd(user: {})", self.user)
    }
}

impl fmt::Display for WhitelistRemoveParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WhitelistRemove(user: {})", self.user)
    }
}
