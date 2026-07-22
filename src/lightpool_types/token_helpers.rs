use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::module::Module;
use crate::lightpool_types::object::ObjectID;

pub const TOKEN_DECIMALS: u32 = 6;
pub const TOKEN_SCALE: u64 = 1_000_000;

const NAMESPACE_ROOT: u8 = 0x00;
const NAMESPACE_TOKEN: u8 = 0x01;
const NAMESPACE_BALANCE: u8 = 0x02;

const MAX_TOKEN_INDEX: u64 = (1u64 << 56) - 1;

const INCREMENT_SLOT: u8 = 0;

fn payload_from_root_slot(slot: u8) -> [u8; ObjectID::PAYLOAD_LENGTH] {
    let mut payload = [0u8; ObjectID::PAYLOAD_LENGTH];
    payload[0] = NAMESPACE_ROOT;
    payload[1] = slot;
    payload
}

fn payload_from_token() -> [u8; ObjectID::PAYLOAD_LENGTH] {
    let mut payload = [0u8; ObjectID::PAYLOAD_LENGTH];
    payload[0] = NAMESPACE_TOKEN;
    payload
}

fn payload_from_balance(account: Address) -> [u8; ObjectID::PAYLOAD_LENGTH] {
    let mut payload = [0u8; ObjectID::PAYLOAD_LENGTH];
    payload[0] = NAMESPACE_BALANCE;
    payload[1..1 + Address::ADDRESS_LENGTH].copy_from_slice(account.as_slice());
    payload
}

/// Default token module contract address (module id + zero salt).
pub fn token_module_contract() -> ContractAddress {
    ContractAddress::new(Module::TOKEN, [0u8; 7])
}

/// Token instance contract. Index starts at 1 and must fit in 7 bytes.
pub fn token_contract(index: u64) -> Result<ContractAddress, String> {
    if index == 0 || index > MAX_TOKEN_INDEX {
        return Err(format!(
            "token contract index must be in 1..={}",
            MAX_TOKEN_INDEX
        ));
    }
    let mut rest = [0u8; 7];
    rest.copy_from_slice(&index.to_be_bytes()[1..]);
    Ok(ContractAddress::new(Module::TOKEN, rest))
}

/// Module-level increment object id.
pub fn increment_object_id() -> ObjectID {
    let contract = token_module_contract();
    ObjectID::generate(contract, payload_from_root_slot(INCREMENT_SLOT))
}

/// Token metadata object id (create/mint input).
pub fn token_object_id(token_contract: ContractAddress) -> ObjectID {
    ObjectID::generate(token_contract, payload_from_token())
}

/// Balance object id for mint/transfer.
pub fn balance_object_id(token_contract: ContractAddress, account: Address) -> ObjectID {
    ObjectID::generate(token_contract, payload_from_balance(account))
}

/// Parse a token contract from CLI: decimal index (>= 1) or 8-byte hex (with optional 0x prefix).
pub fn parse_token_contract(value: &str) -> Result<ContractAddress, String> {
    let trimmed = value.trim();
    let hex_body = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let had_hex_prefix = hex_body.len() < trimmed.len();

    let is_hex = !hex_body.is_empty() && hex_body.chars().all(|c| c.is_ascii_hexdigit());

    // Full 8-byte contract address, e.g. 0200000000000007 or 0x0200000000000007
    if is_hex && hex_body.len() == ContractAddress::CONTRACT_ADDRESS_LENGTH * 2 {
        return decode_token_contract_hex(hex_body);
    }

    // Explicit 0x prefix always means hex, never decimal index.
    if had_hex_prefix && is_hex {
        return decode_token_contract_hex(hex_body);
    }

    // Short decimal token index, e.g. "7"
    if !hex_body.is_empty() && hex_body.chars().all(|c| c.is_ascii_digit()) {
        let index: u64 = hex_body
            .parse()
            .map_err(|e| format!("Invalid token contract index: {}", e))?;
        return token_contract(index);
    }

    if is_hex {
        return decode_token_contract_hex(hex_body);
    }

    Err(format!("Invalid token contract address: {}", value))
}

fn decode_token_contract_hex(value: &str) -> Result<ContractAddress, String> {
    let bytes = hex::decode(value).map_err(|e| format!("Invalid token contract hex: {}", e))?;
    if bytes.len() != ContractAddress::CONTRACT_ADDRESS_LENGTH {
        return Err(format!(
            "token contract must be {} bytes, got {}",
            ContractAddress::CONTRACT_ADDRESS_LENGTH,
            bytes.len()
        ));
    }
    let mut arr = [0u8; ContractAddress::CONTRACT_ADDRESS_LENGTH];
    arr.copy_from_slice(&bytes);
    Ok(ContractAddress::from_bytes(arr))
}
