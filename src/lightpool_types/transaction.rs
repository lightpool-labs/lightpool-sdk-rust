use std::fmt;
use std::hash::Hash;

use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::object::ObjectID;
use crate::lightpool_types::module::Module;
use crate::lightpool_types::crypto::{Digest, Signature};
use crate::lightpool_types::Name;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serialization")]
use bincode;

#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Action {
    pub contract: ContractAddress,
    pub action: Name,
    pub params: Vec<u8>,
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Action")
            .field("contract", &self.contract)
            .field("action", &self.action)
            .field("params", &self.params)
            .finish()
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(contract: {}, params: {:?})",
            self.action, self.contract, self.params)
    }
}

impl Action {
    pub fn new(
        contract: ContractAddress,
        action: Name,
        params: Vec<u8>,
    ) -> Self {
        Self {
            contract,
            action,
            params,
        }
    }

    pub fn contract(&self) -> ContractAddress {
        self.contract
    }

    pub fn module(&self) -> Module {
        self.contract.module()
    }

    pub fn action(&self) -> &Name {
        &self.action
    }

    pub fn params(&self) -> &[u8] {
        &self.params
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Transaction {
    /// Transaction signer. When acting as an agent, this is the agent address.
    pub sender: Address,
    /// Account owner when the sender is an authorized agent.
    #[cfg_attr(feature = "serde", serde(default))]
    pub account: Option<Address>,
    pub expiration: u64,
    pub actions: Vec<Action>,
}

impl Transaction {
    pub fn new(
        sender: Address,
        expiration: u64,
        actions: Vec<Action>,
    ) -> Self {
        Self {
            sender,
            account: None,
            expiration,
            actions,
        }
    }

    pub fn new_with_account(
        sender: Address,
        account: Address,
        expiration: u64,
        actions: Vec<Action>,
    ) -> Self {
        Self {
            sender,
            account: Some(account),
            expiration,
            actions,
        }
    }

    pub fn sender(&self) -> Address {
        self.sender
    }

    /// Returns the account owner for this transaction.
    /// When the sender is an agent, this is the delegated account; otherwise the sender.
    pub fn account(&self) -> Address {
        self.account.unwrap_or(self.sender)
    }

    pub fn is_agent_transaction(&self) -> bool {
        self.account.is_some()
    }

    pub fn expiration(&self) -> u64 {
        self.expiration
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    #[cfg(feature = "serialization")]
    pub fn digest(&self) -> Digest {
        let tx_bytes = bincode::serialize(self).expect("Failed to serialize transaction");
        Digest::new_from_bytes(&tx_bytes)
    }

    #[cfg(not(feature = "serialization"))]
    pub fn digest(&self) -> Digest {
        let mut data = Vec::new();
        data.extend_from_slice(self.sender.as_bytes());
        if let Some(account) = self.account {
            data.extend_from_slice(account.as_bytes());
        }
        data.extend_from_slice(&self.expiration.to_le_bytes());
        Digest::new_from_bytes(&data)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Signature,
}

impl SignedTransaction {
    pub fn new(
        transaction: Transaction,
        signature: Signature,
    ) -> Self {
        Self {
            transaction,
            signature,
        }
    }

    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn inputs(&self) -> Vec<ObjectID> {
        Vec::new()
    }

    pub fn digest(&self) -> Digest {
        Self::calculate_digest(&self.transaction, &self.signature)
    }

    #[cfg(feature = "serialization")]
    fn calculate_digest(transaction: &Transaction, signature: &Signature) -> Digest {
        let tx_bytes = bincode::serialize(transaction).expect("Failed to serialize transaction");

        let mut all_data = tx_bytes;
        let sig_bytes = bincode::serialize(signature).expect("Failed to serialize signature");
        all_data.extend_from_slice(&sig_bytes);

        Digest::new_from_bytes(&all_data)
    }

    #[cfg(not(feature = "serialization"))]
    fn calculate_digest(transaction: &Transaction, _signature: &Signature) -> Digest {
        transaction.digest()
    }
}

/// In-memory verified transaction. Not serializable; use `SignedTransaction` on the wire.
#[derive(Debug, Clone)]
pub struct VerifiedTransaction {
    digest: Digest,
    signed_transaction: SignedTransaction,
}

impl VerifiedTransaction {
    pub fn new(signed_transaction: SignedTransaction) -> Self {
        let digest = signed_transaction.digest();
        Self {
            digest,
            signed_transaction,
        }
    }

    pub fn digest(&self) -> &Digest {
        &self.digest
    }

    pub fn signed_transaction(&self) -> &SignedTransaction {
        &self.signed_transaction
    }

    pub fn transaction(&self) -> &Transaction {
        &self.signed_transaction.transaction
    }

    pub fn inputs(&self) -> Vec<ObjectID> {
        self.signed_transaction.inputs()
    }
}
