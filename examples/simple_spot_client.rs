use lightpool_sdk::{
    LightPoolClient, TransactionBuilder, ActionBuilder, Signer,
    CreateTokenParams,
    extract_token_address_from_events,
    balance_object_id, token_address_from_contract,
    print_receipt_json,
    CreateMarketParams, UpdateMarketParams, PlaceOrderParams, CancelOrderParams,
    OrderSide, TimeInForce, OrderParamsType, MarketState, SideBookSize, TOKEN_SCALE,
    extract_market_address_from_events,
    extract_order_id_from_events, print_spot_receipt_json, spot_market_id,
};
use std::time::Duration;
use env_logger::Env;
use lightpool_sdk::lightpool_types::call::{GetBalance, GetBalanceParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    println!("LightPool Spot Trading Example");
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
            println!("   NOTE: Make sure the LightPool node is running on http://localhost:26300");
            return Ok(());
        }
    }

    // Step 1: Create BTC token
    println!("\nStep 1: Creating BTC token");
    println!("---------------------------");

    let btc_create_params = CreateTokenParams {
        name: "Bitcoin".into(),
        symbol: "BTC".into(),
        total_supply: 21_000_000 * TOKEN_SCALE,
        mintable: true,
        to: trader1_address,
    };

    let btc_create_action = ActionBuilder::create_token(btc_create_params)?;

    let btc_create_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(btc_create_action)
        .build_and_sign_only(&trader1_signer)?;

    let (btc_token_address, btc_balance_id) = match client.submit_transaction(btc_create_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);

            if response.receipt.is_success() {
                let token_address = extract_token_address_from_events(&response.receipt)
                    .expect("Failed to extract BTC token_address");
                let balance_id = balance_object_id(token_address, trader1_address);

                println!("   BTC token created successfully!");
                println!("   Token contract: {}", token_address);
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

    // Step 2: Create USDT token
    println!("\nStep 2: Creating USDT token");
    println!("----------------------------");

    let usdt_create_params = CreateTokenParams {
        name: "USD Tether".into(),
        symbol: "USDT".into(),
        total_supply: 1_000_000_000 * TOKEN_SCALE,
        mintable: true,
        to: trader2_address,
    };

    let usdt_create_action = ActionBuilder::create_token(usdt_create_params)?;

    let usdt_create_tx = TransactionBuilder::new()
        .sender(trader2_address)
        .expiration(u64::MAX)
        .add_action(usdt_create_action)
        .build_and_sign_only(&trader2_signer)?;

    let (usdt_token_address, usdt_balance_id) = match client.submit_transaction(usdt_create_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);

            if response.receipt.is_success() {
                let token_address = extract_token_address_from_events(&response.receipt)
                    .expect("Failed to extract USDT token_address");
                let balance_id = balance_object_id(token_address, trader2_address);

                println!("   USDT token created successfully!");
                println!("   Token contract: {}", token_address);
                (token_address, balance_id)
            } else {
                println!("   ERROR: USDT token creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit USDT creation transaction: {}", e);
            return Ok(());
        }
    };

    // Step 3: Create BTC/USDT market
    println!("\nStep 3: Creating BTC/USDT market");
    println!("----------------------------------");

    let market_create_params = CreateMarketParams {
        name: "BTC/USDT".into(),
        base_token: token_address_from_contract(btc_token_address),
        quote_token: token_address_from_contract(usdt_token_address),
        min_order_size: 100_000,
        tick_size: 1_000_000,
        maker_fee_bps: 10,
        taker_fee_bps: 20,
        allow_market_orders: true,
        state: MarketState::Active,
        limit_order: true,
        side_book_size: SideBookSize::Large,
    };

    let market_create_action = ActionBuilder::create_market(market_create_params)?;

    let market_create_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(market_create_action)
        .build_and_sign_only(&trader1_signer)?;

    let market_address = match client.submit_transaction(market_create_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);

            if response.receipt.is_success() {
                let market_address = extract_market_address_from_events(&response.receipt)
                    .expect("Failed to extract market_address");

                println!("   BTC/USDT market created successfully!");
                println!("   Market Address: {}", market_address);
                market_address
            } else {
                println!("   ERROR: Market creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit market creation transaction: {}", e);
            return Ok(());
        }
    };

    // Step 4: Place sell order (trader1 sells BTC - since trader1 has the initial BTC balance)
    println!("\nStep 4: Placing sell order (trader1 sells BTC)");
    println!("------------------------------------------------");

    let sell_order_params = PlaceOrderParams {
        side: OrderSide::Sell,
        amount: 5_000_000,
        order_type: OrderParamsType::Limit {
            tif: TimeInForce::GTC,
        },
        limit_price: 50_000_000_000,
    };

    let sell_order_action = ActionBuilder::place_order(
        market_address,
        btc_balance_id,
        sell_order_params,
    )?;

    let sell_order_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(sell_order_action)
        .build_and_sign_only(&trader1_signer)?;

    let sell_order_id = match client.submit_transaction(sell_order_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);

            if response.receipt.is_success() {
                let order_id = extract_order_id_from_events(&response.receipt);
                println!("   Sell order placed successfully!");
                if let Some(id) = order_id {
                    println!("   Order ID: {}", id);
                }
                order_id
            } else {
                println!("   ERROR: Sell order placement failed!");
                None
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit sell order transaction: {}", e);
            None
        }
    };

    // Step 5: Place market buy order (trader2 market buys BTC against resting sell)
    println!("\nStep 5: Placing market buy order (trader2 market buys BTC)");
    println!("----------------------------------------------------------");

    let market_buy_params = PlaceOrderParams {
        side: OrderSide::Buy,
        amount: 2_000_000,
        order_type: OrderParamsType::Market {
            slippage: 100,
        },
        limit_price: 50_000_000_000,
    };

    let market_buy_action = ActionBuilder::place_order(
        market_address,
        usdt_balance_id,
        market_buy_params,
    )?;

    let market_buy_tx = TransactionBuilder::new()
        .sender(trader2_address)
        .expiration(u64::MAX)
        .add_action(market_buy_action)
        .build_and_sign_only(&trader2_signer)?;

    match client.submit_transaction(market_buy_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);

            if response.receipt.is_success() {
                println!("   Market buy order executed successfully!");
                println!("   Should match 2 BTC from trader1 resting sell at 50,000 USDT");
            } else {
                println!("   ERROR: Market buy order failed!");
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit market buy order transaction: {}", e);
        }
    }

    // Step 6: Place limit buy order (trader2 buys 2 BTC)
    println!("\nStep 6: Placing limit buy order (trader2 buys 2 BTC)");
    println!("------------------------------------------------------");

    let buy_order_params = PlaceOrderParams {
        side: OrderSide::Buy,
        amount: 2_000_000,
        order_type: OrderParamsType::Limit {
            tif: TimeInForce::GTC,
        },
        limit_price: 50_000_000_000,
    };

    let buy_order_action = ActionBuilder::place_order(
        market_address,
        usdt_balance_id,
        buy_order_params,
    )?;

    let buy_order_tx = TransactionBuilder::new()
        .sender(trader2_address)
        .expiration(u64::MAX)
        .add_action(buy_order_action)
        .build_and_sign_only(&trader2_signer)?;

    match client.submit_transaction(buy_order_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);

            if response.receipt.is_success() {
                let order_id = extract_order_id_from_events(&response.receipt);
                println!("   Buy order placed successfully!");
                if let Some(id) = order_id {
                    println!("   Order ID: {}", id);
                }
                println!("   This should match 2 BTC from trader1 resting sell at 50,000 USDT");
            } else {
                println!("   ERROR: Buy order placement failed!");
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit buy order transaction: {}", e);
        }
    };

    // Step 7: Cancel remaining sell order (trader1 cancels as seller)
    println!("\nStep 7: Cancelling sell order (trader1 cancels as seller)");
    println!("----------------------------------------------------------");

    let Some(sell_order_id) = sell_order_id else {
        println!("   ERROR: No sell order id available to cancel!");
        return Ok(());
    };

    let cancel_order_action = ActionBuilder::cancel_order(
        market_address,
        spot_market_id(market_address),
        CancelOrderParams {
            order_id: sell_order_id,
        },
    )?;

    let cancel_order_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(cancel_order_action)
        .build_and_sign_only(&trader1_signer)?;

    match client.submit_transaction(cancel_order_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);

            if response.receipt.is_success() {
                println!("   Sell order cancelled successfully!");
                println!("   Cancelled order ID: {}", sell_order_id);
            } else {
                println!("   ERROR: Sell order cancellation failed!");
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit cancel order transaction: {}", e);
        }
    }

    // Step 8: Update market parameters
    println!("\nStep 8: Updating market parameters");
    println!("-----------------------------------");

    let market_update_params = UpdateMarketParams {
        min_order_size: Some(50_000),
        maker_fee_bps: Some(5),
        taker_fee_bps: Some(15),
        allow_market_orders: Some(true),
        state: Some(MarketState::Active),
    };

    let market_update_action = ActionBuilder::update_market(
        market_address,
        market_update_params,
    )?;

    let market_update_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(market_update_action)
        .build_and_sign_only(&trader1_signer)?;

    match client.submit_transaction(market_update_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);

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

    println!("\nSpot trading example completed successfully!");
    println!("===============================================");
    println!("Summary of operations:");
    println!("1. Created BTC token (21M supply to trader1)");
    println!("2. Created USDT token (1B supply to trader2)");
    println!("3. Created BTC/USDT trading market");
    println!("4. Placed sell order (trader1 sells 5 BTC at 50,000 USDT)");
    println!("5. Placed market buy order (trader2 market buys 2 BTC)");
    println!("6. Placed limit buy order (trader2 buys 2 BTC)");
    println!("7. Cancelled remaining sell order (trader1 as seller)");
    println!("8. Updated market parameters");
    println!("9. Queried BTC and USDT balances for both traders via call");

    println!("\nStep 9: Querying trader balances via call");
    println!("-----------------------------------------");

    for (token_label, token_contract) in [
        ("btc", btc_token_address),
        ("usdt", usdt_token_address),
    ] {
        for (trader_label, account) in [
            ("trader1", trader1_address),
            ("trader2", trader2_address),
        ] {
            let balance_action = ActionBuilder::get_balance(
                token_contract,
                account,
                GetBalanceParams {},
            )?;

            let balance_tx = TransactionBuilder::new()
                .expiration(u64::MAX)
                .add_action(balance_action)
                .build_and_without_sign()?;

            match client.call(balance_tx).await {
                Ok(bytes) => match bincode::deserialize::<GetBalance>(&bytes) {
                    Ok(balance) => {
                        println!(
                            "{} {} {} {} {}",
                            trader_label,
                            token_label,
                            balance.total / TOKEN_SCALE,
                            balance.locked / TOKEN_SCALE,
                            balance.available / TOKEN_SCALE,
                        );
                    }
                    Err(e) => println!(
                        "   ERROR: Failed to decode {} {} balance: {}",
                        trader_label, token_label, e
                    ),
                },
                Err(e) => println!(
                    "   ERROR: {} {} balance call failed: {}",
                    trader_label, token_label, e
                ),
            }
        }
    }

    Ok(())
}
