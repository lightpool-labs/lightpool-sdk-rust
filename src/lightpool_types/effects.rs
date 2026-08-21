// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use std::collections::HashMap;
use std::fmt;
use crate::lightpool_types::crypto::Digest;
use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::transaction::SignedTransaction;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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

/// Transaction event
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionEvent {
    pub event_type: EventType,
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "crate::lightpool_types::serde_hex::option_contract")
    )]
    pub contract: Option<ContractAddress>,
    pub data: EventData,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EventData {
    Empty,
    String(String),
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::lightpool_types::serde_hex::bytes")
    )]
    Bytes(Vec<u8>),
    Int(i64),
    Map(HashMap<String, String>),
}

impl TransactionEvent {
    /// Create a new event
    pub fn new(
        event_type: EventType,
        contract: Option<ContractAddress>,
        data: EventData,
    ) -> Self {
        Self {
            event_type,
            contract,
            data,
        }
    }

    /// Create a system event
    pub fn system(message: String) -> Self {
        Self {
            event_type: EventType::System,
            contract: None,
            data: EventData::String(message),
        }
    }

    /// Generate event digest
    pub fn digest(&self) -> Digest {
        let event_data = format!("{}:{:?}:{}", self.event_type, self.contract, self.data);
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
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}={}", key, value)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl fmt::Display for TransactionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Event(type: {}, contract: {:?}, data: {})",
            self.event_type, self.contract, self.data
        )
    }
}

/// Calculate digest for TransactionEvents
#[cfg(feature = "serialization")]
pub fn calculate_events_digest(events: &[TransactionEvent]) -> Digest {
    if events.is_empty() {
        return Digest::new_from_bytes(&[]);
    }

    let mut all_data = Vec::new();
    for event in events {
        let event_bytes = bincode::serialize(event).expect("Failed to serialize event");
        all_data.extend_from_slice(&event_bytes);
    }

    Digest::new_from_bytes(&all_data)
}

/// Calculate digest for TransactionEvents - fallback without serialization
#[cfg(not(feature = "serialization"))]
pub fn calculate_events_digest(events: &[TransactionEvent]) -> Digest {
    if events.is_empty() {
        return Digest::new_from_bytes(&[]);
    }

    let mut data = Vec::new();
    data.extend_from_slice(&events.len().to_le_bytes());
    for event in events {
        data.extend_from_slice(format!("{:?}", event.event_type).as_bytes());
    }

    Digest::new_from_bytes(&data)
}

/// Transaction receipt
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionReceipt {
    pub status: ExecutionStatus,
    pub events: Vec<TransactionEvent>,
}

impl TransactionReceipt {
    /// Create a new transaction receipt object
    pub fn new(status: ExecutionStatus, events: Vec<TransactionEvent>) -> Self {
        Self { status, events }
    }

    /// Check if transaction executed successfully
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success)
    }

    /// Create an empty successful transaction receipt
    pub fn empty_success() -> Self {
        Self {
            status: ExecutionStatus::Success,
            events: Vec::new(),
        }
    }

    /// Create a failed transaction receipt
    pub fn failure(error_msg: String) -> Self {
        Self {
            status: ExecutionStatus::Failure(error_msg),
            events: Vec::new(),
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

/// Receipt view of one executed transaction. Does not carry the signed transaction body.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransactionResult {
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::lightpool_types::serde_hex::digest")
    )]
    pub signed_digest: Digest,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::lightpool_types::serde_hex::address")
    )]
    pub sender: Address,
    pub receipt: TransactionReceipt,
}

impl TransactionResult {
    pub fn new(signed_digest: Digest, sender: Address, receipt: TransactionReceipt) -> Self {
        Self {
            signed_digest,
            sender,
            receipt,
        }
    }

    pub fn from_signed(signed: &SignedTransaction, receipt: TransactionReceipt) -> Self {
        Self::new(signed.digest(), signed.transaction().sender(), receipt)
    }

    /// Signed transaction digest (also used as the public tx id on the wire).
    pub fn transaction_digest(&self) -> &Digest {
        &self.signed_digest
    }

    pub fn signed_digest(&self) -> &Digest {
        &self.signed_digest
    }

    pub fn is_success(&self) -> bool {
        self.receipt.is_success()
    }

    pub fn to_receipt(&self) -> TransactionReceipt {
        self.receipt.clone()
    }

    pub fn sender(&self) -> Address {
        self.sender
    }
}
