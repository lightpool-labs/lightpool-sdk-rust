use lightpool_sdk::{
    LightPoolClient, TransactionBuilder, ActionBuilder, Signer,
    Address, CreateTokenParams, MintParams, ObjectID,
    TransactionReceipt,
    print_receipt_json,
};
use std::time::Duration;
use env_logger::Env;
use lightpool_sdk::lightpool_types::address_type::TOKEN_CONTRACT_ADDRESS;
use lightpool_sdk::perp_events::{
    print_perp_receipt_json,
};
use lightpool_sdk::perp_events as perp_ev;
use lightpool_sdk::lightpool_types::{
    CreatePerpMarketParams, UpdatePerpMarketParams, MarketOpenParams, MarketCloseParams,
    PositionSide, OrderType, LeverageMode, PerpMarketState, PerpTimeInForce,
};
use lightpool_sdk::{EventType, EventData};
use lightpool_sdk::{extract_balance_id_from_events, extract_token_id_from_events};

fn perp_contract_address() -> Address {
    let mut bytes = [0u8; 32];
    bytes[0] = 3;
    Address::new(bytes)
}

fn extract_position_id_from_events(receipt: &TransactionReceipt, owner: &Address) -> Option<ObjectID> {
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "perp_position_open" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(ev) = bincode::deserialize::<perp_ev::PerpPositionOpenEvent>(data) {
                        if ev.owner == *owner {
                            return Some(ev.position_id);
                        }
                    }
                }
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("LightPool Perp Trading Example");
    println!("===============================");

    let trader1_signer = Signer::new();
    let trader1_address = trader1_signer.address();

    let trader2_signer = Signer::new();
    let trader2_address = trader2_signer.address();

    println!("Generated Traders:");
    println!("   Trader 1 Address: {}", trader1_address);
    println!("   Trader 2 Address: {}", trader2_address);

    let client = LightPoolClient::new("http://localhost:26300")
        .with_timeout(Duration::from_secs(30));

    println!("\nTesting connection to node...");
    match client.health_check().await {
        Ok(true) => println!("   Node is healthy"),
        Ok(false) => println!("   WARNING: Node responded but not healthy"),
        Err(e) => {
            println!("   ERROR: Failed to connect to node: {}", e);
            println!("   NOTE: Make sure the LightPool node is running on http://localhost:25300");
            return Ok(());
        }
    }

    println!("\nStep 1: Creating BTC token");
    println!("---------------------------");

    let btc_create_params = CreateTokenParams {
        name: "Bitcoin".into(),
        symbol: "BTC".into(),
        total_supply: 21_000_000_000_000,
        mintable: true,
        to: trader1_address,
    };

    let btc_create_action = ActionBuilder::create_token(
        TOKEN_CONTRACT_ADDRESS,
        btc_create_params,
    )?;

    let btc_create_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(btc_create_action)
        .build_and_sign_only(&trader1_signer)?;

    let (btc_token_address, _btc_balance_id) = match client.submit_transaction(btc_create_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let token_address = lightpool_sdk::extract_token_address_from_events(&response.receipt)
                    .expect("Failed to extract BTC token_address");
                let balance_id = lightpool_sdk::extract_balance_id_from_events(&response.receipt)
                    .expect("Failed to extract BTC balance_id");
                println!("   BTC token created successfully!");
                (token_address, balance_id)
            } else {
                println!("   ERROR: BTC token creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit BTC creation transaction: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 2: Creating USDC token");
    println!("----------------------------");

    let usdc_create_params = CreateTokenParams {
        name: "USD Coin".into(),
        symbol: "USDC".into(),
        total_supply: 1_000_000_000_000_000,
        mintable: true,
        to: trader2_address,
    };

    let usdc_create_action = ActionBuilder::create_token(
        TOKEN_CONTRACT_ADDRESS,
        usdc_create_params,
    )?;

    let usdc_create_tx = TransactionBuilder::new()
        .sender(trader2_address)
        .expiration(u64::MAX)
        .add_action(usdc_create_action)
        .build_and_sign_only(&trader2_signer)?;

    let (usdc_token_id, usdc_token_address, trader2_usdc_balance_id) = match client.submit_transaction(usdc_create_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let token_id = extract_token_id_from_events(&response.receipt)
                    .expect("Failed to extract USDC token_id");
                let token_address = lightpool_sdk::extract_token_address_from_events(&response.receipt)
                    .expect("Failed to extract USDC token_address");
                let balance_id = lightpool_sdk::extract_balance_id_from_events(&response.receipt)
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

    println!("\nStep 3: Minting USDC to Trader 1 for collateral");
    println!("-----------------------------------------------");

    let mint_params = MintParams {
        amount: 10_000_000_000,
        to: trader1_address,
    };

    let mint_action = ActionBuilder::mint_token(
        usdc_token_address,
        usdc_token_id,
        mint_params,
    )?;

    let mint_tx = TransactionBuilder::new()
        .sender(trader2_address)
        .expiration(u64::MAX)
        .add_action(mint_action)
        .build_and_sign_only(&trader2_signer)?;

    let trader1_usdc_balance_id = match client.submit_transaction(mint_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let balance_id = extract_balance_id_from_events(&response.receipt)
                    .expect("Failed to extract Trader1 USDC balance_id");
                println!("   Minted USDC to Trader 1 successfully!");
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

    println!("\nStep 4: Creating BTC-PERP market");
    println!("--------------------------------");

    let perp_contract = perp_contract_address();

    let market_create_params = CreatePerpMarketParams {
        name: "BTC-PERP".into(),
        base_token: btc_token_address,
        collateral_token: usdc_token_address,
        min_order_size: 100_000,
        tick_size: 1_000_000,
        maker_fee_bps: 10,
        taker_fee_bps: 20,
        allow_market_orders: true,
        state: PerpMarketState::Active,
        limit_order: true,
        max_leverage: 20,
        maintenance_margin_bps: 250,
        initial_margin_bps: 500,
        funding_interval: 3600,
        liquidation_fee_bps: 50,
        max_position_size: 1_000_000_000,
        max_price_deviation_bps: 500,
        max_funding_rate_bps: 1000,
    };

    let market_create_action = ActionBuilder::create_perp_market(
        perp_contract,
        market_create_params,
    )?;

    let market_create_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(market_create_action)
        .build_and_sign_only(&trader1_signer)?;

    let (market_id, market_address) = match client.submit_transaction(market_create_tx).await {
        Ok(response) => {
            print_perp_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let market_id = perp_ev::extract_perp_market_id_from_events(&response.receipt)
                    .expect("Failed to extract perp market_id");
                let market_address = perp_ev::extract_perp_market_address_from_events(&response.receipt)
                    .expect("Failed to extract perp market_address");
                println!("   BTC-PERP market created successfully!");
                (market_id, market_address)
            } else {
                println!("   ERROR: Perp market creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit perp market creation transaction: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 5: Open LONG for Trader 1");
    println!("------------------------------");
    println!("market_id {}", market_id);
    println!("LONG order: Buy at 50,000 (will match with SHORT at 49,000)");

    let long_open_params = MarketOpenParams {
        position_side: PositionSide::Long,
        size: 1_000_000,
        order_type: OrderType::Limit {
            time_in_force: PerpTimeInForce::GTC,
        },
        limit_price: 50_000_000_000,
        leverage_mode: LeverageMode::Isolated,
        leverage: 10_000,
        collateral: Some(5_000_000_000),
        trigger: None,
        client_order_id: None,
        position_id: None,
    };

    let long_open_action = ActionBuilder::market_open(
        market_address,
        market_id,
        trader1_usdc_balance_id,
        long_open_params,
    )?;

    let long_open_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(long_open_action)
        .build_and_sign_only(&trader1_signer)?;

    match client.submit_transaction(long_open_tx).await {
        Ok(response) => {
            print_perp_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   LONG order submitted successfully! Waiting for SHORT order to match...");
            } else {
                println!("   ERROR: LONG order submission failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit LONG order transaction: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 6: Open SHORT for Trader 2");
    println!("-------------------------------");
    println!("SHORT order: Sell at 49,000 (will match with LONG at 50,000)");
    println!("After this trade, both LONG and SHORT positions should be created");

    let short_open_params = MarketOpenParams {
        position_side: PositionSide::Short,
        size: 1_000_000,
        order_type: OrderType::Limit {
            time_in_force: PerpTimeInForce::GTC,
        },
        limit_price: 49_000_000_000,
        leverage_mode: LeverageMode::Isolated,
        leverage: 8_000,
        collateral: Some(10_000_000_000),
        trigger: None,
        client_order_id: None,
        position_id: None,
    };

    let short_open_action = ActionBuilder::market_open(
        market_address,
        market_id,
        trader2_usdc_balance_id,
        short_open_params,
    )?;

    let short_open_tx = TransactionBuilder::new()
        .sender(trader2_address)
        .expiration(u64::MAX)
        .add_action(short_open_action)
        .build_and_sign_only(&trader2_signer)?;

    let (trader2_position_id, trader1_position_id) = match client.submit_transaction(short_open_tx).await {
        Ok(response) => {
            print_perp_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                // After SHORT order, both positions should be created from the matching trade
                let short_pos_id = extract_position_id_from_events(&response.receipt, &trader2_address)
                    .expect("Failed to extract SHORT position_id for trader2");
                println!("   SHORT opened successfully! Position: {}", short_pos_id);
                
                // The LONG position should also be created from the same trade
                let long_pos_id = extract_position_id_from_events(&response.receipt, &trader1_address)
                    .expect("Failed to extract LONG position_id for trader1");
                println!("   LONG position also created from matching trade! Position: {}", long_pos_id);
                
                (short_pos_id, long_pos_id)
            } else {
                println!("   ERROR: SHORT open failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit SHORT open transaction: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 7: Close LONG for Trader 1");
    println!("-------------------------------");
    println!("Closing LONG position: {}", trader1_position_id);

    let long_close_params = MarketCloseParams {
        size: 1_000_000,
        order_type: OrderType::Limit {
            time_in_force: PerpTimeInForce::GTC,
        },
        limit_price: 51_000_000_000,
        trigger: None,
        client_order_id: None,
        position_id: trader1_position_id,
    };

    let long_close_action = ActionBuilder::market_close(
        market_address,
        market_id,
        trader1_position_id,
        long_close_params,
    )?;

    let long_close_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(long_close_action)
        .build_and_sign_only(&trader1_signer)?;

    match client.submit_transaction(long_close_tx).await {
        Ok(response) => {
            print_perp_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   LONG closed successfully!");
            } else {
                println!("   ERROR: LONG close failed!");
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit LONG close transaction: {}", e);
        }
    }

    println!("\nStep 8: Close SHORT for Trader 2");
    println!("--------------------------------");

    let short_close_params = MarketCloseParams {
        size: 1_000_000,
        order_type: OrderType::Limit {
            time_in_force: PerpTimeInForce::GTC,
        },
        limit_price:51_000_000_000,
        trigger: None,
        client_order_id: None,
        position_id: trader2_position_id,
    };

    let short_close_action = ActionBuilder::market_close(
        market_address,
        market_id,
        trader2_position_id,
        short_close_params,
    )?;

    let short_close_tx = TransactionBuilder::new()
        .sender(trader2_address)
        .expiration(u64::MAX)
        .add_action(short_close_action)
        .build_and_sign_only(&trader2_signer)?;

    match client.submit_transaction(short_close_tx).await {
        Ok(response) => {
            print_perp_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   SHORT closed successfully!");
            } else {
                println!("   ERROR: SHORT close failed!");
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit SHORT close transaction: {}", e);
        }
    }

    println!("\nStep 9: Update market parameters");
    println!("--------------------------------");

    let market_update_params = UpdatePerpMarketParams {
        min_order_size: Some(50_000),
        maker_fee_bps: Some(5),
        taker_fee_bps: Some(15),
        allow_market_orders: Some(true),
        state: None,
        max_leverage: Some(25),
        maintenance_margin_bps: Some(200),
        initial_margin_bps: Some(400),
        funding_interval: Some(1800),
        liquidation_fee_bps: Some(40),
        max_position_size: Some(2_000_000_000),
        max_price_deviation_bps: Some(400),
        max_funding_rate_bps: Some(900),
    };

    let market_update_action = ActionBuilder::update_perp_market(
        market_address,
        market_id,
        market_update_params,
    )?;

    let market_update_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(market_update_action)
        .build_and_sign_only(&trader1_signer)?;

    match client.submit_transaction(market_update_tx).await {
        Ok(response) => {
            print_perp_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   Market updated successfully!");
            } else {
                println!("   ERROR: Market update failed!");
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit market update transaction: {}", e);
        }
    }

    println!("\nPerp trading example completed successfully!");
    println!("===========================================");

    Ok(())
} 