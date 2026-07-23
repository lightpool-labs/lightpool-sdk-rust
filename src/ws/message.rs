// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use serde::{Deserialize, Serialize};

use crate::lightpool_types::block::ReceiptBlock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Execution-stage receipts (early).
    NewBlock(ReceiptBlock),
    /// Commit-stage receipts (finalized).
    ReceiptBlock(ReceiptBlock),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Subscription {
    NewBlocks,
    ReceiptBlocks,
}
