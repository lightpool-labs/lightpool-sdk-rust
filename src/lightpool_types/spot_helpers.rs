use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::contract::ContractAddress;
use crate::lightpool_types::module::Module;
use crate::lightpool_types::object::ObjectID;

pub const INCREMENT_SLOT: u32 = 0;
pub const MARKET_SLOT: u32 = 1;
pub const BIDS_SLOT: u32 = 2;
pub const ASKS_SLOT: u32 = 3;

const NAMESPACE_ROOT: u8 = 0x00;

const MAX_MARKET_INDEX: u64 = (1u64 << 56) - 1;

/// Default spot module contract address (module id + zero salt).
pub fn spot_module_contract() -> ContractAddress {
    ContractAddress::new(Module::SPOT, [0u8; 7])
}

/// Market instance contract from increment index (starts at 1).
pub fn market_contract(index: u64) -> Result<ContractAddress, String> {
    if index == 0 || index > MAX_MARKET_INDEX {
        return Err(format!(
            "market contract index must be in 1..={}",
            MAX_MARKET_INDEX
        ));
    }
    let mut rest = [0u8; 7];
    rest.copy_from_slice(&index.to_be_bytes()[1..]);
    Ok(ContractAddress::new(Module::SPOT, rest))
}

fn payload_from_root_slot(slot: u8) -> [u8; ObjectID::PAYLOAD_LENGTH] {
    let mut payload = [0u8; ObjectID::PAYLOAD_LENGTH];
    payload[0] = NAMESPACE_ROOT;
    payload[1] = slot;
    payload
}

fn spot_object_id(
    market_address: ContractAddress,
    payload: [u8; ObjectID::PAYLOAD_LENGTH],
) -> ObjectID {
    ObjectID::generate(market_address, payload)
}

/// Market metadata object id.
pub fn spot_market_id(market_address: ContractAddress) -> ObjectID {
    spot_object_id(
        market_address,
        payload_from_root_slot(MARKET_SLOT as u8),
    )
}

/// Bids side book object id.
pub fn spot_bids_id(market_address: ContractAddress) -> ObjectID {
    spot_object_id(
        market_address,
        payload_from_root_slot(BIDS_SLOT as u8),
    )
}

/// Asks side book object id.
pub fn spot_asks_id(market_address: ContractAddress) -> ObjectID {
    spot_object_id(
        market_address,
        payload_from_root_slot(ASKS_SLOT as u8),
    )
}

/// Parse a spot market contract from CLI: decimal index (>= 1) or 8-byte hex.
pub fn parse_market_contract(value: &str) -> Result<ContractAddress, String> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()) {
        let index: u64 = value
            .parse()
            .map_err(|e| format!("Invalid market contract index: {}", e))?;
        return market_contract(index);
    }

    let bytes = hex::decode(value).map_err(|e| format!("Invalid market contract hex: {}", e))?;
    if bytes.len() != ContractAddress::CONTRACT_ADDRESS_LENGTH {
        return Err(format!(
            "market contract must be {} bytes, got {}",
            ContractAddress::CONTRACT_ADDRESS_LENGTH,
            bytes.len()
        ));
    }
    let mut arr = [0u8; ContractAddress::CONTRACT_ADDRESS_LENGTH];
    arr.copy_from_slice(&bytes);
    Ok(ContractAddress::from_bytes(arr))
}

/// Convert token contract address to the Address form used in market params.
pub fn token_address_from_contract(contract: ContractAddress) -> Address {
    contract.to_address()
}
