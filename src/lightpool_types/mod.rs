// Copyright (c) LightPool Labs
// Author: xiaoyu1998

pub mod address_type;
pub mod contract;
pub mod crypto;
pub mod module;
pub mod object;
pub mod transaction;
pub mod eip712;
pub mod effects;
pub mod token_actions;
pub mod token_helpers;
pub mod spot_actions;
pub mod spot_helpers;
pub mod event_contract_actions;
pub mod event_contract_helpers;
pub mod staking_actions;
pub mod staking_helpers;
pub mod vault_actions;
pub mod vault_helpers;
pub mod margin_actions;
pub mod margin_helpers;
pub mod bridge_actions;
pub mod bridge_helpers;
pub mod account_actions;
pub mod account_helpers;
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
pub use transaction::{Action, AuthScheme, Transaction, SignedTransaction, VerifiedTransaction};
pub use eip712::{
    eip712_hash_for_tx_digest, LIGHTPOOL_EIP712_CHAIN_ID, LIGHTPOOL_EIP712_NAME,
    LIGHTPOOL_EIP712_VERIFYING_CONTRACT, LIGHTPOOL_EIP712_VERSION,
};
pub use effects::{
    TransactionReceipt, TransactionEvent, EventType, EventData, ExecutionStatus,
    TransactionEffect, TransactionResult
};
pub use block::ReceiptBlock;
pub use token_actions::{
    CreateTokenParams, MintParams, TransferParams,
};
pub use token_helpers::{
    TOKEN_DECIMALS, TOKEN_SCALE,
    token_module_contract, token_contract, increment_object_id,
    token_object_id, balance_object_id, parse_token_contract,
};
pub use spot_actions::{
    CreateMarketParams, UpdateMarketParams, ClaimMarketFeesParams, PlaceOrderParams,
    PlaceOrderGroupParams, AttachedTriggerParams, ParentOrderType,
    CancelOrderParams, UpdateOrderParams, SubmitOraclePriceParams,
    OrderSide, TimeInForce, OrderParamsType, MarketState, SegmentSize,
    LimitOrderParams, TriggerOrderParams, MarketOrderParams, TriggerType,
};
pub use spot_helpers::{
    spot_module_contract, market_contract, spot_market_id, spot_bids_id, spot_asks_id,
    spot_oracle_quotes_id, parse_market_contract, token_address_from_contract,
    INCREMENT_SLOT as SPOT_INCREMENT_SLOT,
    MARKET_SLOT, BIDS_SLOT, ASKS_SLOT, ORACLE_QUOTES_SLOT,
};
pub use event_contract_actions::{
    CreateEventContractParams, MintEventContractParams, BurnEventContractParams,
    ResolveEventContractParams, RedeemEventContractParams,
    EventContractState,
};
pub use event_contract_helpers::event_contract_module_contract;
pub use staking_actions::{
    AllocateStakeParams, BondLplParams, DeallocateStakeParams, InitStakingConfigParams,
    PromoteParams, RegisterValidatorParams, StakePurpose, UnbondLplParams, WithdrawUnbondParams,
};
pub use staking_helpers::staking_module_contract;
pub use vault_actions::{
    CloseVaultParams, CreateVaultParams, DepositVaultParams, SetVaultAllowDepositParams,
    SetVaultManagerParams, WithdrawVaultParams,
};
pub use vault_helpers::{vault_account, vault_contract, vault_module_contract, vault_portfolio_id};
pub use margin_actions::{
    BorrowParams, CreateMarginParams, CreatePoolParams, DepositCollateralParams, LiquidateParams,
    RepayParams, SupplyParams, WithdrawCollateralParams, WithdrawSupplyParams,
    MARGIN_MODE_CROSS, MARGIN_MODE_ISOLATED,
};
pub use margin_helpers::{
    is_margin_account_contract, is_pool_contract, margin_account_contract, margin_module_contract,
    margin_trading_account, pool_account, pool_contract, TAG_ACCOUNT, TAG_POOL,
};
pub use bridge_actions::{
    BridgeAuthority, BridgeDepositMessage, BridgeVote, BridgeWithdrawParams, ConfirmDepositParams,
    InitBridgeConfigParams,
};
pub use bridge_helpers::bridge_module_contract;
pub use account_actions::SetAgentParams;
pub use account_helpers::account_module_contract;
pub use order_id::{OrderId, parse_order_id};
pub use order_id_type::OrderIdType;
pub use name_type::Name;

use crate::name;
pub const CREATE_ACTION: Name = name!("create");
pub const MINT_ACTION: Name = name!("mint");
pub const TRANSFER_ACTION: Name = name!("transfer");
pub const CREATE_MARKET_ACTION: Name = name!("mkt_create");
pub const UPDATE_MARKET_ACTION: Name = name!("mkt_update");
pub const CLAIM_MARKET_FEES_ACTION: Name = name!("mkt_claim");
pub const PLACE_ORDER_ACTION: Name = name!("ord_place");
pub const PLACE_ORDER_GROUP_ACTION: Name = name!("ord_group");
pub const CANCEL_ORDER_ACTION: Name = name!("ord_cancel");
pub const UPDATE_ORDER_ACTION: Name = name!("ord_update");
pub const SUBMIT_ORACLE_PRICE_ACTION: Name = name!("ora_submit");
pub const EC_CREATE_ACTION: Name = name!("ec_create");
pub const EC_MINT_ACTION: Name = name!("ec_mint");
pub const EC_BURN_ACTION: Name = name!("ec_burn");
pub const EC_RESOLVE_ACTION: Name = name!("ec_resolve");
pub const EC_REDEEM_ACTION: Name = name!("ec_redeem");
pub const INIT_CONFIG_ACTION: Name = name!("init_config");
pub const BOND_LPL_ACTION: Name = name!("bond_lpl");
pub const UNBOND_LPL_ACTION: Name = name!("unbond_lpl");
pub const PROM_PENDING_ACTION: Name = name!("prom_pending");
pub const PROM_RUNNING_ACTION: Name = name!("prom_running");
pub const ALLOCATE_STAKE_ACTION: Name = name!("allocate");
pub const DEALLOCATE_STAKE_ACTION: Name = name!("deallocate");
pub const WITHDRAW_UNBOND_ACTION: Name = name!("wd_unbond");
pub const REGISTER_VALIDATOR_ACTION: Name = name!("reg_val");
pub const MARKET_INFO_ACTION: Name = name!("mkt_info");
pub const TOKEN_INFO_ACTION: Name = name!("token_info");
pub const GET_BALANCE_ACTION: Name = name!("get_balance");
pub const VAULT_CREATE_ACTION: Name = name!("create");
pub const VAULT_DEPOSIT_ACTION: Name = name!("deposit");
pub const VAULT_WITHDRAW_ACTION: Name = name!("withdraw");
pub const VAULT_SET_MANAGER_ACTION: Name = name!("set_manager");
pub const VAULT_SET_ALLOW_ACTION: Name = name!("set_allow");
pub const VAULT_CLOSE_ACTION: Name = name!("close");
pub const VAULT_GET_PORTFOLIO_ACTION: Name = name!("get_pf");
pub const MARGIN_CREATE_POOL_ACTION: Name = name!("crt_pool");
pub const MARGIN_SUPPLY_ACTION: Name = name!("supply");
pub const MARGIN_WITHDRAW_SUPPLY_ACTION: Name = name!("wd_sup");
pub const MARGIN_CREATE_ACCOUNT_ACTION: Name = name!("crt_mgn");
pub const MARGIN_DEPOSIT_ACTION: Name = name!("deposit");
pub const MARGIN_WITHDRAW_ACTION: Name = name!("wd_col");
pub const MARGIN_BORROW_ACTION: Name = name!("borrow");
pub const MARGIN_REPAY_ACTION: Name = name!("repay");
pub const MARGIN_LIQUIDATE_ACTION: Name = name!("liqd");
pub const BRIDGE_CONFIRM_DEP_ACTION: Name = name!("confirm_dep");
pub const BRIDGE_WITHDRAW_ACTION: Name = name!("withdraw");
pub const BRIDGE_REFUND_WD_ACTION: Name = name!("refund_wd");
pub const SET_AGENT_ACTION: Name = name!("set_agent");
