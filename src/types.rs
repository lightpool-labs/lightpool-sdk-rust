use serde::{Deserialize, Serialize};
use crate::lightpool_types::{TransactionReceipt, SignedTransaction, TransactionEvent, ExecutionStatus};
use hex;

/// Response from submitting a transaction
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubmitTransactionResponse {
    /// The digest of the transaction
    pub digest: String,
    /// The receipt of executing the transaction
    pub receipt: TransactionReceipt,
}

/// Parameters for submitting a transaction via RPC
#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTransactionParams {
    /// Verified transaction
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

/// A display-friendly version of TransactionReceipt with hex-encoded transaction digest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayTransactionReceipt {
    /// Transaction digest as hex string
    pub transaction_digest: String,
    /// Transaction execution status
    pub status: ExecutionStatus,
    /// Transaction events
    pub events: Vec<TransactionEvent>,
    /// Block number where transaction was included
    pub block_num: u64,
}

impl From<TransactionReceipt> for DisplayTransactionReceipt {
    fn from(receipt: TransactionReceipt) -> Self {
        Self {
            transaction_digest: hex::encode(receipt.transaction_digest.as_bytes()),
            status: receipt.status,
            events: receipt.events,
            block_num: receipt.block_num,
        }
    }
}

impl DisplayTransactionReceipt {
    /// Check if transaction executed successfully
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }
    
    /// Get transaction digest as hex string
    pub fn digest_hex(&self) -> &str {
        &self.transaction_digest
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