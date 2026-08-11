// Copyright (c) LightPool Labs
// Author: xiaoyu1998

/// LightPool SDK for interacting with the LightPool blockchain
pub mod lightpool_types;
pub mod transaction;
pub mod token_events;
pub mod spot_events;
pub mod event_contract_events;
pub mod staking_events;
pub mod vault_events;
pub mod margin_events;
// pub mod perp_events;
// pub mod spark_events;
pub mod client;
pub mod types;
pub mod crypto;
pub mod error;
pub mod websocket_client;
pub mod ws;

pub use client::LightPoolClient;
pub use transaction::{TransactionBuilder, ActionBuilder};
pub use crypto::Signer;
pub use error::{SdkError, SdkResult};
pub use websocket_client::WebSocketClient;
pub use ws::message::{Subscription, Message};

pub use lightpool_types::{
    Address, ContractAddress, Module, ObjectID, Transaction, VerifiedTransaction,
    TransactionEvent, EventType, EventData, ExecutionStatus,
    TransactionEffect, Action, Digest, PublicKey, SecretKey, Signature,
    CreateTokenParams, MintParams, TransferParams,
    TOKEN_DECIMALS, TOKEN_SCALE,
    token_module_contract, token_contract, increment_object_id,
    token_object_id, balance_object_id, parse_token_contract,
    CreateMarketParams, UpdateMarketParams, PlaceOrderParams, PlaceOrderGroupParams,
    AttachedTriggerParams, ParentOrderType, CancelOrderParams, UpdateOrderParams,
    SubmitOraclePriceParams, TriggerType,
    OrderSide, TimeInForce, OrderParamsType, MarketState, SegmentSize,
    spot_module_contract, spot_market_id, token_address_from_contract,
    CreateEventContractParams, MintEventContractParams, BurnEventContractParams,
    ResolveEventContractParams, RedeemEventContractParams,
    InitStakingConfigParams, BondLplParams, UnbondLplParams, PromoteParams,
    AllocateStakeParams, DeallocateStakeParams, WithdrawUnbondParams, RegisterValidatorParams,
    StakePurpose,
    CreateVaultParams, DepositVaultParams, WithdrawVaultParams, SetVaultManagerParams,
    SetVaultAllowDepositParams, CloseVaultParams,
    CreatePoolParams, SupplyParams, WithdrawSupplyParams, CreateMarginParams,
    DepositCollateralParams, WithdrawCollateralParams, BorrowParams, RepayParams, LiquidateParams,
    MARGIN_MODE_CROSS, MARGIN_MODE_ISOLATED,
    InitBridgeConfigParams, ConfirmDepositParams, BridgeWithdrawParams, BridgeAuthority,
    BridgeDepositMessage, BridgeVote, SetAgentParams,
    event_contract_module_contract, staking_module_contract,
    vault_module_contract, vault_contract, vault_account, vault_portfolio_id,
    margin_module_contract, pool_contract, margin_account_contract, pool_account,
    margin_trading_account, TAG_POOL, TAG_ACCOUNT,
    bridge_module_contract, account_module_contract,
    OrderId, OrderIdType,
};

pub use lightpool_types::TransactionReceipt;
pub use lightpool_types::ReceiptBlock;

pub use token_events::*;
pub use spot_events::*;
pub use event_contract_events::*;
pub use staking_events::*;
pub use vault_events::*;
pub use margin_events::*;
pub use types::{
    SubmitTransactionParams, SubmitTransactionResponse,
    RpcRequest, RpcResponse, RpcError,
    DisplayTransactionReceipt,
};
