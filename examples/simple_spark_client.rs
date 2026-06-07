use lightpool_sdk::{
    LightPoolClient, TransactionBuilder, ActionBuilder, Signer,
    Address, CreateTokenParams, MintParams, ObjectID,
    TransactionReceipt,
    print_receipt_json,
};
use std::time::Duration;
use env_logger::Env;
use lightpool_sdk::lightpool_types::address_type::TOKEN_CONTRACT_ADDRESS;
use lightpool_sdk::spark_events::{
    print_spark_receipt_json,
    PoolCreatedEvent as SparkPoolCreatedEvent,
};
use lightpool_sdk::lightpool_types::{
    SparkCreatePoolParams, SparkBuyParams, SparkSellParams,
};
use lightpool_sdk::{EventType, EventData};
use lightpool_sdk::{
    extract_token_id_from_events,
    extract_balance_id_from_events,
    extract_token_address_from_events,
    extract_pool_id_from_events,
    extract_pool_address_from_events,
    extract_spark_token_address_from_events,
};

fn spark_contract_address() -> Address {
    let mut bytes = [0u8; 32];
    bytes[0] = 5; // Module::SPARK
    Address::new(bytes)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("LightPool Spark Example (Create Pool, Buy, Sell)");
    println!("=================================================");

    let creator_signer = Signer::new();
    let creator_address = creator_signer.address();

    let trader_signer = Signer::new();
    let trader_address = trader_signer.address();

    println!("Generated Addresses:");
    println!("   Creator: {}", creator_address);
    println!("   Trader:  {}", trader_address);

    let client = LightPoolClient::new("http://localhost:26300")
        .with_timeout(Duration::from_secs(30));

    println!("\nStep 1: Creating USDC token");
    println!("---------------------------");

    let usdc_params = CreateTokenParams {
        name: "USD Coin".into(),
        symbol: "USDC".into(),
        total_supply: 1_000_000_000_000_000, // 1,000,000 USDC (6 decimals)
        mintable: true,
        to: creator_address,
    };

    let usdc_create_action = ActionBuilder::create_token(
        TOKEN_CONTRACT_ADDRESS,
        usdc_params,
    )?;

    let usdc_create_tx = TransactionBuilder::new()
        .sender(creator_address)
        .expiration(u64::MAX)
        .add_action(usdc_create_action)
        .build_and_sign_only(&creator_signer)?;

    let (usdc_token_id, usdc_token_address, creator_usdc_balance_id) = match client.submit_transaction(usdc_create_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let token_id = extract_token_id_from_events(&response.receipt)
                    .expect("Failed to extract USDC token_id");
                let token_address = extract_token_address_from_events(&response.receipt)
                    .expect("Failed to extract USDC token_address");
                let balance_id = extract_balance_id_from_events(&response.receipt)
                    .expect("Failed to extract USDC balance_id");
                println!("   USDC token created successfully!");
                (token_id, token_address, balance_id)
            } else {
                println!("   ERROR: USDC token creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit USDC creation transaction: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 2: Creating Trump pool (Spark create_pool)");
    println!("----------------------------------------------");

    let spark_contract = spark_contract_address();

    let create_pool_params = SparkCreatePoolParams {
        quote: usdc_token_address,
        name: "Trump".into(),
        symbol: "Trump".into(),
        total_supply: 1_000_000_000, // 1000 Trump (6 decimals)
        amount: 100_000_000,           // initial token amount to buy (100.0 Trump)
        max_quote_input: 300_000_000_000, // max quote to spend (200,000 USDC)
        initial_virtual_token_reserves: 1_000_000_000,     // 1000 Trump
        initial_virtual_quote_reserves: 2_000_000_000_000, // 2,000,000 USDC
        market_cap_limit: 100_000_000_000_000,             // 100,000,000 USDC
    };

    let create_pool_action = ActionBuilder::spark_create_pool(
        spark_contract,
        create_pool_params,
        creator_usdc_balance_id,
    )?;

    let create_pool_tx = TransactionBuilder::new()
        .sender(creator_address)
        .expiration(u64::MAX)
        .add_action(create_pool_action)
        .build_and_sign_only(&creator_signer)?;

    let (pool_address, pool_id, eth_token_address) = match client.submit_transaction(create_pool_tx).await {
        Ok(response) => {
            print_spark_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let pool_address = extract_pool_address_from_events(&response.receipt)
                    .expect("Failed to extract pool_address");
                let pool_id = extract_pool_id_from_events(&response.receipt)
                    .expect("Failed to extract pool_id");
                let eth_token_address = extract_spark_token_address_from_events(&response.receipt)
                    .expect("Failed to extract Trump token address from pool_created event");
                println!("   Trump pool created successfully! Pool: {}", pool_id);
                (pool_address, pool_id, eth_token_address)
            } else {
                println!("   ERROR: Trump pool creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit Trump pool creation transaction: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 3: Mint USDC to Trader for buying Trump");
    println!("-------------------------------------------");

    let mint_params = MintParams {
        amount: 50_000_000_000, // 50,000 USDC
        to: trader_address,
    };

    let mint_action = ActionBuilder::mint_token(
        usdc_token_address,
        usdc_token_id,
        mint_params,
    )?;

    let mint_tx = TransactionBuilder::new()
        .sender(creator_address)
        .expiration(u64::MAX)
        .add_action(mint_action)
        .build_and_sign_only(&creator_signer)?;

    let trader_usdc_balance_id = match client.submit_transaction(mint_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let balance_id = extract_balance_id_from_events(&response.receipt)
                    .expect("Failed to extract Trader USDC balance_id");
                println!("   Minted USDC to Trader successfully!");
                balance_id
            } else {
                println!("   ERROR: USDC mint failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit USDC mint transaction: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 4: Trader buys Trump from pool");
    println!("----------------------------------");

    let buy_params = SparkBuyParams {
        amount: 1_000_000,        // 1.0 Trump
        max_quote_input: 3_000_000_000, // up to 2,000 USDC
    };

    let buy_action = ActionBuilder::spark_buy(
        pool_address,
        pool_id,
        trader_usdc_balance_id,
        buy_params,
    )?;

    let buy_tx = TransactionBuilder::new()
        .sender(trader_address)
        .expiration(u64::MAX)
        .add_action(buy_action)
        .build_and_sign_only(&trader_signer)?;

    let trader_eth_balance_id = match client.submit_transaction(buy_tx).await {
        Ok(response) => {
            print_spark_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                // Extract the trader's received Trump balance id from Transfer events
                let bal_id = lightpool_sdk::spark_events::extract_spark_balance_id_from_events(&response.receipt, &trader_address)
                    .expect("Failed to extract trader Trump balance_id");
                println!("   Trader bought Trump successfully! Token balance: {}", bal_id);
                bal_id
            } else {
                println!("   ERROR: Trump buy failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit Trump buy transaction: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 5: Trader sells part of Trump back to pool");
    println!("----------------------------------------------");

    let sell_params = SparkSellParams {
        amount: 500_000,         // 0.5 Trump
        min_quote_output: 300_000_000, // expect at least 300 USDC
    };

    let sell_action = ActionBuilder::spark_sell(
        pool_address,
        pool_id,
        trader_eth_balance_id,
        sell_params,
    )?;

    let sell_tx = TransactionBuilder::new()
        .sender(trader_address)
        .expiration(u64::MAX)
        .add_action(sell_action)
        .build_and_sign_only(&trader_signer)?;

    match client.submit_transaction(sell_tx).await {
        Ok(response) => {
            print_spark_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   Trader sold Trump successfully!");
            } else {
                println!("   ERROR: Trump sell failed!");
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit Trump sell transaction: {}", e);
        }
    }

    println!("\nSpark example completed.");
    println!("========================");

    Ok(())
} 