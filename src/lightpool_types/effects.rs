use std::collections::HashMap;
use std::fmt;
use crate::lightpool_types::crypto::Digest;
use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::transaction::VerifiedTransaction;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg(feature = "serialization")]
use bincode;

/// Execution status, indicating the result of transaction execution
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ExecutionStatus {
    /// Execution successful
    Success,
    /// Execution failed, with error message
    Failure(String),
}

/// Event type enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EventType {
    /// System event
    System,
    /// Transfer event
    Transfer,
    /// Call event
    Call(String),
    /// Custom event
    Custom(String),
}

/// Transaction event, representing events emitted during transaction execution
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionEvent {
    /// Event type
    pub event_type: EventType,
    /// Event sender
    pub sender: Option<Address>,
    /// Event contract
    pub contract: Option<ContractAddress>,
    /// Event timestamp
    pub block_num: u64,
    /// Event data
    pub data: EventData,
}

/// Event data, can be different types
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EventData {
    /// Empty event data
    Empty,
    /// String data
    String(String),
    /// Byte data
    Bytes(Vec<u8>),
    /// Integer data
    Int(i64),
    /// Key-value pair data
    Map(HashMap<String, String>),
}

impl TransactionEvent {
    /// Create a new event
    pub fn new(
        event_type: EventType,
        sender: Option<Address>,
        contract: Option<ContractAddress>,
        block_num: u64,
        data: EventData,
    ) -> Self {
        Self {
            event_type,
            sender,
            contract,
            block_num,
            data,
        }
    }

    /// Create a system event
    pub fn system(message: String, block_num: u64) -> Self {
        Self {
            event_type: EventType::System,
            sender: None,
            contract: None,
            block_num,
            data: EventData::String(message),
        }
    }

    /// Generate event digest
    pub fn digest(&self) -> Digest {
        let event_data = format!("{}:{}:{:?}:{:?}:{}", 
            self.event_type, self.block_num, 
            self.sender, self.contract, self.data);
        Digest::new_from_bytes(event_data.as_bytes())
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::System => write!(f, "System"),
            EventType::Transfer => write!(f, "Transfer"),
            EventType::Call(action) => write!(f, "Call({})", action),
            EventType::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

impl fmt::Display for EventData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventData::Empty => write!(f, "Empty"),
            EventData::String(s) => write!(f, "String({})", s),
            EventData::Bytes(bytes) => write!(f, "Bytes({} bytes)", bytes.len()),
            EventData::Int(i) => write!(f, "Int({})", i),
            EventData::Map(map) => {
                write!(f, "Map(")?;
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}={}", key, value)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl fmt::Display for TransactionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Event(type: {}, ts: {}, sender: {:?}, contract: {:?}, data: {})", 
               self.event_type, self.block_num, self.sender, self.contract, self.data)
    }
}

/// Calculate digest for TransactionEvents
#[cfg(feature = "serialization")]
pub fn calculate_events_digest(events: &[TransactionEvent]) -> Digest {
    if events.is_empty() {
        // If no events, return digest of empty data
        return Digest::new_from_bytes(&[]);
    }
    
    // Use bincode to serialize all events
    let mut all_data = Vec::new();
    for event in events {
        let event_bytes = bincode::serialize(event).expect("Failed to serialize event");
        all_data.extend_from_slice(&event_bytes);
    }
    
    // Use new_from_bytes method
    Digest::new_from_bytes(&all_data)
}

/// Calculate digest for TransactionEvents - fallback without serialization
#[cfg(not(feature = "serialization"))]
pub fn calculate_events_digest(events: &[TransactionEvent]) -> Digest {
    if events.is_empty() {
        // If no events, return digest of empty data
        return Digest::new_from_bytes(&[]);
    }
    
    // Simple hash based on event count and types when serialization is not available
    let mut data = Vec::new();
    data.extend_from_slice(&events.len().to_le_bytes());
    for event in events {
        data.extend_from_slice(&event.block_num.to_le_bytes());
    }
    
    Digest::new_from_bytes(&data)
}

/// TransactionReceipt contains the basic receipt information for a transaction
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionReceipt {
    /// Transaction digest
    pub transaction_digest: Digest,
    /// Transaction status code, indicating whether execution was successful
    pub status: ExecutionStatus,
    /// Transaction events
    pub events: Vec<TransactionEvent>,
    /// Block number where transaction was included
    pub block_num: u64,
}

impl TransactionReceipt {
    /// Create a new transaction receipt object
    pub fn new(
        transaction_digest: Digest,
        status: ExecutionStatus,
        events: Vec<TransactionEvent>,
        block_num: u64,
    ) -> Self {
        Self {
            transaction_digest,
            status,
            events,
            block_num,
        }
    }
    
    /// Check if transaction executed successfully
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }

    /// Create an empty successful transaction receipt
    pub fn empty_success(transaction_digest: Digest, block_num: u64) -> Self {
        Self {
            transaction_digest,
            status: ExecutionStatus::Success,
            events: Vec::new(),
            block_num,
        }
    }

    /// Create a failed transaction receipt
    pub fn failure(transaction_digest: Digest, error_msg: String, block_num: u64) -> Self {
        Self {
            transaction_digest,
            status: ExecutionStatus::Failure(error_msg),
            events: Vec::new(),
            block_num,
        }
    }

    /// Get events digest
    pub fn events_digest(&self) -> Digest {
        calculate_events_digest(&self.events)
    }

    /// Check if has events
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

/// Placeholder for TransactionEffect - can be expanded later
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionEffect {
    /// Transaction digest
    pub transaction_digest: Digest,
    /// Status
    pub status: ExecutionStatus,
}

impl TransactionEffect {
    pub fn new(transaction_digest: Digest, status: ExecutionStatus) -> Self {
        Self {
            transaction_digest,
            status,
        }
    }
} 

/// TransactionResult represents the execution result of a transaction
/// without the dirty data cache, suitable for external consumption
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionResult {
    /// The verified transaction
    pub transaction: VerifiedTransaction,
    /// The transaction receipt
    pub receipt: TransactionReceipt,
}

impl TransactionResult {
    /// Create a new TransactionResult
    pub fn new(
        transaction: VerifiedTransaction,
        receipt: TransactionReceipt,
    ) -> Self {
        Self {
            transaction,
            receipt,
        }
    }

    /// Get the transaction digest
    pub fn transaction_digest(&self) -> &Digest {
        self.transaction.digest()
    }

    /// Check if the transaction executed successfully
    pub fn is_success(&self) -> bool {
        self.receipt.is_success()
    }

    /// Convert to transaction receipt
    pub fn to_receipt(&self) -> TransactionReceipt {
        self.receipt.clone()
    }

    /// Get the transaction sender
    pub fn sender(&self) -> Address {
        self.transaction.transaction().sender()
    }

    /// Get all input object IDs for the transaction
    pub fn inputs(&self) -> Vec<crate::lightpool_types::object::ObjectID> {
        self.transaction.inputs()
    }
} 