use serde::{Deserialize, Serialize};
use crate::lightpool_types::block::VerifiedBlock;
use crate::lightpool_types::address_type::Address;
use crate::lightpool_types::object::ObjectID;
// use crate::lightpool_types::OrderId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    NewBlock(VerifiedBlock),
    User(Vec<UserUpdate>),
    Trades(Vec<Trade>),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Subscription {
    NewBlocks,
    User(Address),
    Trades(Address),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UserUpdate {
    Fill(TradeInfo),
    Transfer(Transfer),
    OrderCreated(OrderInfo),
    // OrderCancel(OrderId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeInfo {
    pub market: String,
    pub side: String,
    pub price: String,
    pub size: String,
    pub remaining_size: String,
    pub time: u64,
    pub hash: String,
    pub start_position: String,
    pub dir: String,
    pub closed_pnl: String,
    pub order_id: String,
    pub custom_id: Option<String>,
    pub crossed: bool,
    pub fee: String,
    pub fee_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub token: String,
    pub amount: String,
    pub from: Address,
    pub to: Address,
    pub balance_id: ObjectID,
    pub remainder_id: Option<ObjectID>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInfo {
    pub order_id: String,
    pub side: String,
    pub size: String,
    pub market: Address,
    pub order_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub market: String,
    pub side: String,
    pub price: String,
    pub size: String,
    pub time: u64,
    pub hash: String,
    pub order_id: String,
    pub users: (String, String),
} 