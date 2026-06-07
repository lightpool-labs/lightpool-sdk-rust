use crate::lightpool_types::crypto::Digest;
use crate::lightpool_types::effects::TransactionResult;
use serde::{Deserialize, Serialize};

/// VerifiedBlock represents a verified block that has been validated and contains
/// the block data along with all transaction results from execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedBlock {
    /// Block number
    pub block_num: u64,
    /// Digest of the block
    pub digest: Digest,
    /// Vector of transaction results from block execution
    pub transaction_outputs: Vec<TransactionResult>,
}

impl VerifiedBlock {
    /// Create a new VerifiedBlock
    pub fn new(
        block_num: u64,
        digest: Digest,
        transaction_outputs: Vec<TransactionResult>,
    ) -> Self {
        Self {
            block_num,
            digest,
            transaction_outputs,
        }
    }

    /// Get the block digest
    pub fn digest(&self) -> &Digest {
        &self.digest
    }

    /// Get the transaction results
    pub fn transaction_outputs(&self) -> &[TransactionResult] {
        &self.transaction_outputs
    }

    /// Get the block number
    pub fn block_num(&self) -> u64 {
        self.block_num
    }

    /// Get the number of transactions in this block
    pub fn transaction_count(&self) -> usize {
        self.transaction_outputs.len()
    }

    /// Check if the block contains any transactions
    pub fn is_empty(&self) -> bool {
        self.transaction_outputs.is_empty()
    }

    /// Get all successful transaction results
    pub fn successful_transactions(&self) -> Vec<&TransactionResult> {
        self.transaction_outputs
            .iter()
            .filter(|output| output.is_success())
            .collect()
    }

    /// Get all failed transaction results
    pub fn failed_transactions(&self) -> Vec<&TransactionResult> {
        self.transaction_outputs
            .iter()
            .filter(|output| !output.is_success())
            .collect()
    }
} 