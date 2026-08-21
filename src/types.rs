// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use serde::{Deserialize, Serialize};
use crate::lightpool_types::{TransactionReceipt, TransactionEvent, ExecutionStatus};

/// Response from submitting a transaction
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubmitTransactionResponse {
    /// The digest of the transaction
    pub digest: String,
    /// Block number where the transaction was included
    pub block_num: u64,
    /// The receipt of executing the transaction
    pub receipt: TransactionReceipt,
}

/// Parameters for submitting a transaction via RPC
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionParams {
    /// Signed transaction
    pub tx: crate::lightpool_types::SignedTransaction,
}

/// RPC request structure
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest<T> {
    pub jsonrpc: String,
    pub method: String,
    pub params: T,
    pub id: u64,
}

/// RPC response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub result: Option<T>,
    pub error: Option<RpcError>,
    pub id: u64,
}

/// RPC error structure
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// A display-friendly version of TransactionReceipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayTransactionReceipt {
    /// Transaction execution status
    pub status: ExecutionStatus,
    /// Transaction events
    pub events: Vec<TransactionEvent>,
}

impl From<TransactionReceipt> for DisplayTransactionReceipt {
    fn from(receipt: TransactionReceipt) -> Self {
        Self {
            status: receipt.status,
            events: receipt.events,
        }
    }
}

impl DisplayTransactionReceipt {
    /// Check if transaction executed successfully
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Check if has events
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }
}

impl<T> RpcRequest<T> {
    pub fn new(method: String, params: T, id: u64) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id,
        }
    }
}
