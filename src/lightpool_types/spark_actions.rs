use crate::lightpool_types::address_type::Address;
use compact_str::CompactString;
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePoolParams {
    pub quote: Address,                          // Quote token address (e.g., USDC)
    pub name: CompactString,                     // Token name
    pub symbol: CompactString,                   // Token symbol
    pub total_supply: u64,                       // Total token supply
    pub amount: u64,                             // Initial token amount to buy
    pub max_quote_input: u64,                    // Max quote to spend for initial buy
    pub initial_virtual_token_reserves: u64,     // Initial virtual token reserves
    pub initial_virtual_quote_reserves: u64,     // Initial virtual quote reserves
    pub market_cap_limit: u64,                   // Market cap limit to complete
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyParams {
    pub amount: u64,                             // Token amount to buy
    pub max_quote_input: u64,                    // Max quote to spend
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellParams {
    pub amount: u64,                             // Token amount to sell
    pub min_quote_output: u64,                   // Min quote to receive
}

impl fmt::Display for CreatePoolParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CreatePool(name: {}, symbol: {}, quote: {}, total_supply: {}, amount: {}, max_input: {}, virtual_reserves: {}/{}, mcap_limit: {})",
            self.name,
            self.symbol,
            self.quote,
            self.total_supply,
            self.amount,
            self.max_quote_input,
            self.initial_virtual_token_reserves,
            self.initial_virtual_quote_reserves,
            self.market_cap_limit
        )
    }
}

impl fmt::Display for BuyParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Buy(amount: {}, max_input: {})", self.amount, self.max_quote_input)
    }
}

impl fmt::Display for SellParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sell(amount: {}, min_output: {})", self.amount, self.min_quote_output)
    }
} 