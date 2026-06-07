pub mod address_type;
pub mod contract;
pub mod crypto;
pub mod module;
pub mod object;
pub mod transaction;
pub mod effects;
pub mod token_actions;
pub mod token_helpers;
pub mod spot_actions;
pub mod spot_helpers;
pub mod order_id;
pub mod order_id_type;
pub mod name_type;
pub mod block;
pub mod call;

pub use address_type::Address;
pub use contract::ContractAddress;
pub use module::Module;
pub use crypto::{Digest, PublicKey, SecretKey, Signature, generate_production_keypair, derive_public_key_from_secret};
pub use object::ObjectID;
pub use transaction::{Action, Transaction, SignedTransaction, VerifiedTransaction};
pub use effects::{
    TransactionReceipt, TransactionEvent, EventType, EventData, ExecutionStatus,
    TransactionEffect, TransactionResult
};
pub use block::VerifiedBlock;
pub use token_actions::{
    CreateTokenParams, MintParams, TransferParams,
};
pub use token_helpers::{
    TOKEN_DECIMALS, TOKEN_SCALE,
    token_module_contract, token_contract, increment_object_id,
    token_object_id, balance_object_id, parse_token_contract,
};
pub use spot_actions::{
    CreateMarketParams, UpdateMarketParams, PlaceOrderParams, CancelOrderParams,
    OrderSide, TimeInForce, OrderParamsType, MarketState, SideBookSize,
    LimitOrderParams, TriggerOrderParams, MarketOrderParams, TriggerType,
};
pub use spot_helpers::{
    spot_module_contract, market_contract, spot_market_id, spot_bids_id, spot_asks_id,
    parse_market_contract, token_address_from_contract,
    INCREMENT_SLOT as SPOT_INCREMENT_SLOT,
    MARKET_SLOT, BIDS_SLOT, ASKS_SLOT,
};
pub use order_id::{OrderId, parse_order_id};
pub use order_id_type::OrderIdType;
pub use name_type::Name;

use crate::name;
pub const CREATE_ACTION: Name = name!("create");
pub const MINT_ACTION: Name = name!("mint");
pub const TRANSFER_ACTION: Name = name!("transfer");
pub const CREATE_MARKET_ACTION: Name = name!("mkt_create");
pub const UPDATE_MARKET_ACTION: Name = name!("mkt_update");
pub const PLACE_ORDER_ACTION: Name = name!("ord_place");
pub const CANCEL_ORDER_ACTION: Name = name!("ord_cancel");
pub const MARKET_INFO_ACTION: Name = name!("mkt_info");
pub const TOKEN_INFO_ACTION: Name = name!("token_info");
pub const GET_BALANCE_ACTION: Name = name!("get_balance");
