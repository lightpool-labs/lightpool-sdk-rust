use crate::lightpool_types::{Transaction, SignedTransaction, VerifiedTransaction, Action};
use crate::lightpool_types::Address;
use crate::lightpool_types::ContractAddress;
use crate::lightpool_types::ObjectID;
use crate::lightpool_types::{
    CreateTokenParams, MintParams, TransferParams,
    CreateMarketParams, UpdateMarketParams, PlaceOrderParams, CancelOrderParams,
    balance_object_id, token_module_contract, spot_module_contract, spot_market_id,
    token_object_id,
};
use crate::lightpool_types::call::{
    GetMarketInfoParams, GetOrderBookParams, GetTokenInfoParams, GetBalanceParams,
    MARKET_INFO_ACTION, ORDER_BOOK_ACTION, TOKEN_INFO_ACTION, GET_BALANCE_ACTION,
};
use crate::lightpool_types::{
    Name, CREATE_ACTION, MINT_ACTION, TRANSFER_ACTION,
    CREATE_MARKET_ACTION, UPDATE_MARKET_ACTION, PLACE_ORDER_ACTION, CANCEL_ORDER_ACTION,
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
            vec![signature],
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
        Ok(SignedTransaction::new(tx, vec![]))
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
            vec![],
            token_module_contract(),
            CREATE_ACTION,
            serialized_params,
        ))
    }

    pub fn mint_token(
        contract: ContractAddress,
        token_id: ObjectID,
        params: MintParams,
    ) -> SdkResult<Action> {
        let to_balance_id = balance_object_id(contract, params.to);
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![token_id, to_balance_id],
            contract,
            MINT_ACTION,
            serialized_params,
        ))
    }

    pub fn transfer_token(
        contract: ContractAddress,
        balance_id: ObjectID,
        params: TransferParams,
    ) -> SdkResult<Action> {
        let to_balance_id = balance_object_id(contract, params.to);
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![balance_id, to_balance_id],
            contract,
            TRANSFER_ACTION,
            serialized_params,
        ))
    }

    pub fn create_market(params: CreateMarketParams) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![],
            spot_module_contract(),
            CREATE_MARKET_ACTION,
            serialized_params,
        ))
    }

    pub fn update_market(
        market_contract: ContractAddress,
        params: UpdateMarketParams,
    ) -> SdkResult<Action> {
        let market_id = spot_market_id(market_contract);
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![market_id],
            market_contract,
            UPDATE_MARKET_ACTION,
            serialized_params,
        ))
    }

    pub fn place_order(
        market_contract: ContractAddress,
        balance_id: ObjectID,
        params: PlaceOrderParams,
    ) -> SdkResult<Action> {
        let market_id = spot_market_id(market_contract);
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![market_id, balance_id],
            market_contract,
            PLACE_ORDER_ACTION,
            serialized_params,
        ))
    }

    pub fn cancel_order(
        market_contract: ContractAddress,
        market_id: ObjectID,
        params: CancelOrderParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![market_id],
            market_contract,
            CANCEL_ORDER_ACTION,
            serialized_params,
        ))
    }

    pub fn get_market_info(
        market_contract: ContractAddress,
        market_id: ObjectID,
        params: GetMarketInfoParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![market_id],
            market_contract,
            MARKET_INFO_ACTION,
            serialized_params,
        ))
    }

    pub fn get_orderbook(
        market_contract: ContractAddress,
        market_id: ObjectID,
        params: GetOrderBookParams,
    ) -> SdkResult<Action> {
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![market_id],
            market_contract,
            ORDER_BOOK_ACTION,
            serialized_params,
        ))
    }

    pub fn get_token_info(
        token_contract: ContractAddress,
        params: GetTokenInfoParams,
    ) -> SdkResult<Action> {
        let token_id = token_object_id(token_contract);
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![token_id],
            token_contract,
            TOKEN_INFO_ACTION,
            serialized_params,
        ))
    }

    pub fn get_balance(
        token_contract: ContractAddress,
        account: Address,
        params: GetBalanceParams,
    ) -> SdkResult<Action> {
        let balance_id = balance_object_id(token_contract, account);
        let serialized_params = bincode::serialize(&params)?;
        Ok(Action::new(
            vec![balance_id],
            token_contract,
            GET_BALANCE_ACTION,
            serialized_params,
        ))
    }

    pub fn custom_action(
        inputs: Vec<ObjectID>,
        contract: ContractAddress,
        action_name: Name,
        params: Vec<u8>,
    ) -> Action {
        Action::new(inputs, contract, action_name, params)
    }
}
