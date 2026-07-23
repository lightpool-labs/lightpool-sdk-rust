// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use crate::lightpool_types::{Transaction, SignedTransaction, VerifiedTransaction, Action, Signature};
use crate::lightpool_types::Address;
use crate::lightpool_types::ContractAddress;
use crate::lightpool_types::{
    CreateTokenParams, MintParams, TransferParams,
    CreateMarketParams, UpdateMarketParams, PlaceOrderParams, CancelOrderParams, UpdateOrderParams,
    CreateEventContractParams, MintEventContractParams, BurnEventContractParams,
    ResolveEventContractParams, RedeemEventContractParams,
    InitStakingConfigParams, BondLplParams, UnbondLplParams, PromoteParams,
    token_module_contract, spot_module_contract, event_contract_module_contract,
    staking_module_contract,
};
use crate::lightpool_types::call::{
    GetMarketInfoParams, GetOrderBookParams, GetTokenInfoParams, GetBalanceParams,
    MARKET_INFO_ACTION, ORDER_BOOK_ACTION, TOKEN_INFO_ACTION, GET_BALANCE_ACTION,
};
use crate::lightpool_types::{
    Name, CREATE_ACTION, MINT_ACTION, TRANSFER_ACTION,
    CREATE_MARKET_ACTION, UPDATE_MARKET_ACTION, PLACE_ORDER_ACTION, CANCEL_ORDER_ACTION,
    UPDATE_ORDER_ACTION,
    EC_CREATE_ACTION, EC_MINT_ACTION, EC_BURN_ACTION, EC_RESOLVE_ACTION, EC_REDEEM_ACTION,
    INIT_CONFIG_ACTION, BOND_LPL_ACTION, UNBOND_LPL_ACTION, PROM_PENDING_ACTION, PROM_RUNNING_ACTION,
};
use crate::crypto::Signer;
use crate::error::{SdkError, SdkResult};

/// Builder for constructing transactions
pub struct TransactionBuilder {
    sender: Option<Address>,
    account: Option<Address>,
    expiration: u64,
    actions: Vec<Action>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            sender: None,
            account: None,
            expiration: 0,
            actions: Vec::new(),
        }
    }

    pub fn sender(mut self, sender: Address) -> Self {
        self.sender = Some(sender);
        self
    }

    pub fn account(mut self, account: Address) -> Self {
        self.account = Some(account);
        self
    }

    pub fn expiration(mut self, expiration: u64) -> Self {
        self.expiration = expiration;
        self
    }

    pub fn add_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    pub fn build(self) -> SdkResult<Transaction> {
        let sender = self.sender.ok_or_else(|| SdkError::Transaction("Sender not set".to_string()))?;

        if self.actions.is_empty() {
            return Err(SdkError::Transaction("No actions provided".to_string()));
        }

        Ok(Transaction {
            sender,
            account: self.account,
            expiration: self.expiration,
            actions: self.actions,
        })
    }

    pub fn build_and_sign_only(self, signer: &Signer) -> SdkResult<SignedTransaction> {
        let transaction = self.build()?;
        let digest = transaction.digest();
        let signature = signer.sign_transaction(&digest)?;

        Ok(SignedTransaction::new(
            transaction,
            signature,
        ))
    }

    pub fn build_and_verify(self, signer: &Signer) -> SdkResult<VerifiedTransaction> {
        let signed_tx = self.build_and_sign_only(signer)?;
        Ok(VerifiedTransaction::new(signed_tx))
    }

    pub fn build_and_without_sign(self) -> SdkResult<SignedTransaction> {
        let sender = self.sender.unwrap_or_else(|| Address::zero());

        if self.actions.is_empty() {
            return Err(SdkError::Transaction("No actions provided".to_string()));
        }

        let tx = Transaction {
            sender,
            account: self.account,
            expiration: self.expiration,
            actions: self.actions,
        };
        Ok(SignedTransaction::new(tx, Signature::default()))
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing actions
pub struct ActionBuilder;

impl ActionBuilder {
    pub fn create_token(params: CreateTokenParams) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            token_module_contract(),
            CREATE_ACTION,
            serialized_params,
        ))
    }

    pub fn mint_token(
        contract: ContractAddress,
        params: MintParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            contract,
            MINT_ACTION,
            serialized_params,
        ))
    }

    pub fn transfer_token(
        contract: ContractAddress,
        params: TransferParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            contract,
            TRANSFER_ACTION,
            serialized_params,
        ))
    }

    pub fn create_market(params: CreateMarketParams) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            spot_module_contract(),
            CREATE_MARKET_ACTION,
            serialized_params,
        ))
    }

    pub fn update_market(
        market_contract: ContractAddress,
        params: UpdateMarketParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            UPDATE_MARKET_ACTION,
            serialized_params,
        ))
    }

    pub fn place_order(
        market_contract: ContractAddress,
        params: PlaceOrderParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            PLACE_ORDER_ACTION,
            serialized_params,
        ))
    }

    pub fn cancel_order(
        market_contract: ContractAddress,
        params: CancelOrderParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            CANCEL_ORDER_ACTION,
            serialized_params,
        ))
    }

    pub fn update_order(
        market_contract: ContractAddress,
        params: UpdateOrderParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            UPDATE_ORDER_ACTION,
            serialized_params,
        ))
    }

    pub fn create_event_contract(params: CreateEventContractParams) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            event_contract_module_contract(),
            EC_CREATE_ACTION,
            serialized_params,
        ))
    }

    pub fn mint_event_contract(
        market_contract: ContractAddress,
        params: MintEventContractParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            EC_MINT_ACTION,
            serialized_params,
        ))
    }

    pub fn burn_event_contract(
        market_contract: ContractAddress,
        params: BurnEventContractParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            EC_BURN_ACTION,
            serialized_params,
        ))
    }

    pub fn resolve_event_contract(
        market_contract: ContractAddress,
        params: ResolveEventContractParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            EC_RESOLVE_ACTION,
            serialized_params,
        ))
    }

    pub fn redeem_event_contract(
        market_contract: ContractAddress,
        params: RedeemEventContractParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            EC_REDEEM_ACTION,
            serialized_params,
        ))
    }

    pub fn init_staking_config(params: InitStakingConfigParams) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            staking_module_contract(),
            INIT_CONFIG_ACTION,
            serialized_params,
        ))
    }

    pub fn prom_pending(params: PromoteParams) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            staking_module_contract(),
            PROM_PENDING_ACTION,
            serialized_params,
        ))
    }

    pub fn prom_running(params: PromoteParams) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            staking_module_contract(),
            PROM_RUNNING_ACTION,
            serialized_params,
        ))
    }

    pub fn bond_lpl(params: BondLplParams) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            staking_module_contract(),
            BOND_LPL_ACTION,
            serialized_params,
        ))
    }

    pub fn unbond_lpl(params: UnbondLplParams) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            staking_module_contract(),
            UNBOND_LPL_ACTION,
            serialized_params,
        ))
    }

    pub fn get_market_info(
        market_contract: ContractAddress,
        params: GetMarketInfoParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            MARKET_INFO_ACTION,
            serialized_params,
        ))
    }

    pub fn get_orderbook(
        market_contract: ContractAddress,
        params: GetOrderBookParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            market_contract,
            ORDER_BOOK_ACTION,
            serialized_params,
        ))
    }

    pub fn get_token_info(
        token_contract: ContractAddress,
        params: GetTokenInfoParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            token_contract,
            TOKEN_INFO_ACTION,
            serialized_params,
        ))
    }

    pub fn get_balance(
        token_contract: ContractAddress,
        _account: Address,
        params: GetBalanceParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            token_contract,
            GET_BALANCE_ACTION,
            serialized_params,
        ))
    }

    pub fn custom_action(
        contract: ContractAddress,
        action_name: Name,
        params: Vec<u8>,
    ) -> Action {
        Action::new(contract, action_name, params)
    }
}
