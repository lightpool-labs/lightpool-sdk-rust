// Copyright (c) LightPool Labs
// Author: xiaoyu1998

//! Spot TP/SL sample: standalone Sell trigger orders and Buy order-group (normalTpsl).
//!
//! Run against a local node:
//! `cargo run --example simple_spot_tpsl_client`

use env_logger::Env;
use lightpool_sdk::lightpool_types::call::{GetBalance, GetBalanceParams};
use lightpool_sdk::{
    balance_object_id, extract_market_address_from_events, extract_order_id_from_events,
    extract_token_address_from_events, print_receipt_json, print_spot_receipt_json, ActionBuilder,
    Address, AttachedTriggerParams, CancelOrderParams, ContractAddress, CreateMarketParams,
    CreateTokenParams, LightPoolClient, MarketState, OrderParamsType, OrderSide, ParentOrderType,
    PlaceOrderGroupParams, PlaceOrderParams, SegmentSize, Signer,
    TimeInForce, TransactionBuilder, TransferParams, TriggerType, TOKEN_SCALE,
};
use std::time::Duration;

const TICK_SIZE: u64 = 1_000_000;
const PARENT_PRICE: u64 = 50_000_000_000;
const SL_TRIGGER_PRICE: u64 = 45_000_000_000;
const TP_TRIGGER_PRICE: u64 = 55_000_000_000;
const TRIGGER_LIMIT_PRICE: u64 = 40_000_000_000;
const ORDER_AMOUNT: u64 = 1_000_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("LightPool Spot TP/SL Example");
    println!("============================");

    let trader1_signer = Signer::new();
    let trader1_address = trader1_signer.address();
    let trader2_signer = Signer::new();
    let trader2_address = trader2_signer.address();

    println!("Generated Traders:");
    println!("   Trader 1 (trigger + order-group buyer): {}", trader1_address);
    println!("   Trader 2 (CLOB liquidity seller):       {}", trader2_address);

    let client = LightPoolClient::new("http://localhost:26300").with_timeout(Duration::from_secs(30));

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

    // -------------------------------------------------------------------------
    // Setup: tokens, cross-fund, market
    // -------------------------------------------------------------------------
    println!("\nStep 1: Creating BTC token (to trader1)");
    println!("---------------------------------------");
    let btc_token_address = create_token(
        &client,
        &trader1_signer,
        trader1_address,
        "Bitcoin",
        "BTC",
        21_000_000 * TOKEN_SCALE,
    )
    .await?;

    println!("\nStep 2: Creating USDT token (to trader2)");
    println!("----------------------------------------");
    let usdt_token_address = create_token(
        &client,
        &trader2_signer,
        trader2_address,
        "USD Tether",
        "USDT",
        1_000_000_000 * TOKEN_SCALE,
    )
    .await?;

    println!("\nStep 3: Funding traders for TP/SL flows");
    println!("---------------------------------------");
    // trader2 needs BTC to rest a sell that fills the Buy order-group parent.
    transfer_token(
        &client,
        &trader1_signer,
        trader1_address,
        btc_token_address,
        trader2_address,
        10 * TOKEN_SCALE,
    )
    .await?;
    // trader1 needs USDT to place the Buy order-group parent.
    transfer_token(
        &client,
        &trader2_signer,
        trader2_address,
        usdt_token_address,
        trader1_address,
        1_000_000 * TOKEN_SCALE,
    )
    .await?;

    println!("\nStep 4: Creating BTC/USDT market");
    println!("--------------------------------");
    let market_address = create_market(
        &client,
        &trader1_signer,
        trader1_address,
        btc_token_address,
        usdt_token_address,
    )
    .await?;

    // -------------------------------------------------------------------------
    // Standalone Sell trigger (rests in TpslBook, locks base)
    // -------------------------------------------------------------------------
    println!("\nStep 5: Placing standalone Sell SL trigger (trader1)");
    println!("----------------------------------------------------");
    println!("   Spot triggers are Sell-only; they rest in TpslBook (not CLOB).");

    let sl_trigger_params = PlaceOrderParams {
        side: OrderSide::Sell,
        amount: ORDER_AMOUNT,
        order_type: OrderParamsType::Trigger {
            trigger_price: SL_TRIGGER_PRICE,
            is_market: false,
            trigger_type: TriggerType::SL,
        },
        limit_price: TRIGGER_LIMIT_PRICE,
        token_address: btc_token_address,
    };

    let sl_trigger_action = ActionBuilder::place_order(market_address, sl_trigger_params)?;
    let sl_trigger_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(sl_trigger_action)
        .build_and_sign_only(&trader1_signer)?;

    let sl_trigger_id = match client.submit_transaction(sl_trigger_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let order_id = extract_order_id_from_events(&response.receipt);
                println!("   Sell SL trigger placed (order rests until mark <= trigger).");
                if let Some(id) = order_id {
                    println!("   Order ID: {}", id);
                }
                order_id
            } else {
                println!("   ERROR: Sell SL trigger placement failed!");
                None
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit SL trigger: {}", e);
            None
        }
    };

    println!("\nStep 6: Placing standalone Sell TP trigger (trader1)");
    println!("----------------------------------------------------");

    let tp_trigger_params = PlaceOrderParams {
        side: OrderSide::Sell,
        amount: ORDER_AMOUNT,
        order_type: OrderParamsType::Trigger {
            trigger_price: TP_TRIGGER_PRICE,
            is_market: true,
            trigger_type: TriggerType::TP,
        },
        limit_price: TRIGGER_LIMIT_PRICE,
        token_address: btc_token_address,
    };

    let tp_trigger_action = ActionBuilder::place_order(market_address, tp_trigger_params)?;
    let tp_trigger_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(tp_trigger_action)
        .build_and_sign_only(&trader1_signer)?;

    match client.submit_transaction(tp_trigger_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let order_id = extract_order_id_from_events(&response.receipt);
                println!("   Sell TP trigger placed (activates when mark >= trigger).");
                if let Some(id) = order_id {
                    println!("   Order ID: {}", id);
                }
            } else {
                println!("   ERROR: Sell TP trigger placement failed!");
            }
        }
        Err(e) => println!("   ERROR: Failed to submit TP trigger: {}", e),
    }

    // -------------------------------------------------------------------------
    // Order group (normalTpsl): Buy parent + pending TP/SL, arm after full fill
    // -------------------------------------------------------------------------
    println!("\nStep 7: Resting ask for order-group parent fill (trader2)");
    println!("-------------------------------------------------------");

    let ask_params = PlaceOrderParams {
        side: OrderSide::Sell,
        amount: ORDER_AMOUNT,
        order_type: OrderParamsType::Limit {
            tif: TimeInForce::GTC,
        },
        limit_price: PARENT_PRICE,
        token_address: btc_token_address,
    };
    let ask_action = ActionBuilder::place_order(market_address, ask_params)?;
    let ask_tx = TransactionBuilder::new()
        .sender(trader2_address)
        .expiration(u64::MAX)
        .add_action(ask_action)
        .build_and_sign_only(&trader2_signer)?;

    match client.submit_transaction(ask_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   Resting sell placed at parent price.");
            } else {
                println!("   ERROR: Resting sell failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit resting sell: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 8: Placing Buy order-group with TP+SL (trader1)");
    println!("---------------------------------------------------");
    println!("   Parent must be Buy. TP/SL arm into TpslBook only after parent fully fills.");
    println!("   OCO pair shares one base lock (TP holds the lock).");

    let group_params = PlaceOrderGroupParams {
        side: OrderSide::Buy,
        amount: ORDER_AMOUNT,
        limit_price: PARENT_PRICE,
        token_address: usdt_token_address,
        parent_type: ParentOrderType::Limit {
            tif: TimeInForce::GTC,
        },
        tp: Some(AttachedTriggerParams {
            trigger_price: TP_TRIGGER_PRICE,
            limit_price: TRIGGER_LIMIT_PRICE,
            is_market: false,
        }),
        sl: Some(AttachedTriggerParams {
            trigger_price: SL_TRIGGER_PRICE,
            limit_price: TRIGGER_LIMIT_PRICE,
            is_market: false,
        }),
    };

    let group_action = ActionBuilder::place_order_group(market_address, group_params)?;
    let group_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(group_action)
        .build_and_sign_only(&trader1_signer)?;

    match client.submit_transaction(group_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   Order-group parent filled; attached TP/SL should now rest in TpslBook.");
            } else {
                println!("   ERROR: Order-group placement failed!");
            }
        }
        Err(e) => println!("   ERROR: Failed to submit order-group: {}", e),
    }

    // -------------------------------------------------------------------------
    // Optional oracle submit (needs running committee membership)
    // -------------------------------------------------------------------------
    println!("\nStep 9: Optional oracle price submit (may fail without committee stake)");
    println!("----------------------------------------------------------------------");
    println!("   Mark follows oracle finalize at block end; TP/SL activate from mark.");

    let oracle_action =
        ActionBuilder::submit_oracle_price_for_market(market_address, SL_TRIGGER_PRICE)?;
    let oracle_tx = TransactionBuilder::new()
        .sender(trader1_address)
        .expiration(u64::MAX)
        .add_action(oracle_action)
        .build_and_sign_only(&trader1_signer)?;

    match client.submit_transaction(oracle_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   Oracle quote submitted. Finalize at block end may trigger SL.");
            } else {
                println!("   Oracle submit rejected (expected if sender is not a committee validator).");
            }
        }
        Err(e) => {
            println!("   Oracle submit failed (expected without staking committee): {}", e);
        }
    }

    // -------------------------------------------------------------------------
    // Cancel standalone SL trigger
    // -------------------------------------------------------------------------
    if let Some(order_id) = sl_trigger_id {
        println!("\nStep 10: Cancelling standalone SL trigger (trader1)");
        println!("---------------------------------------------------");
        let cancel_action = ActionBuilder::cancel_order(
            market_address,
            CancelOrderParams { order_id },
        )?;
        let cancel_tx = TransactionBuilder::new()
            .sender(trader1_address)
            .expiration(u64::MAX)
            .add_action(cancel_action)
            .build_and_sign_only(&trader1_signer)?;

        match client.submit_transaction(cancel_tx).await {
            Ok(response) => {
                print_spot_receipt_json(&response.receipt);
                if response.receipt.is_success() {
                    println!("   Standalone SL trigger cancelled; base unlocked.");
                } else {
                    println!("   ERROR: Cancel failed (order may already be gone).");
                }
            }
            Err(e) => println!("   ERROR: Failed to submit cancel: {}", e),
        }
    }

    println!("\nStep 11: Querying final balances");
    println!("--------------------------------");
    print_trader_balances(
        &client,
        btc_token_address,
        usdt_token_address,
        trader1_address,
        trader2_address,
    )
    .await;

    println!("\nSpot TP/SL example finished.");
    println!("============================");
    println!("Summary:");
    println!("1. Created BTC/USDT market");
    println!("2. Placed standalone Sell SL + Sell TP triggers (TpslBook)");
    println!("3. Filled Buy order-group parent and armed OCO TP/SL");
    println!("4. Attempted oracle submit (committee-gated)");
    println!("5. Cancelled standalone SL trigger");

    Ok(())
}

async fn create_token(
    client: &LightPoolClient,
    signer: &Signer,
    to: Address,
    name: &str,
    symbol: &str,
    total_supply: u64,
) -> Result<ContractAddress, Box<dyn std::error::Error>> {
    let params = CreateTokenParams {
        name: name.into(),
        symbol: symbol.into(),
        total_supply,
        mintable: true,
        to,
    };
    let action = ActionBuilder::create_token(params)?;
    let tx = TransactionBuilder::new()
        .sender(to)
        .expiration(u64::MAX)
        .add_action(action)
        .build_and_sign_only(signer)?;

    let response = client.submit_transaction(tx).await?;
    print_receipt_json(&response.receipt);
    if !response.receipt.is_success() {
        return Err(format!("{} token creation failed", symbol).into());
    }
    let token_address = extract_token_address_from_events(&response.receipt)
        .ok_or_else(|| format!("Failed to extract {} token address", symbol))?;
    println!("   {} token: {}", symbol, token_address);
    let _ = balance_object_id(token_address, to);
    Ok(token_address)
}

async fn transfer_token(
    client: &LightPoolClient,
    signer: &Signer,
    sender: Address,
    token: ContractAddress,
    to: Address,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let action = ActionBuilder::transfer_token(token, TransferParams { to, amount })?;
    let tx = TransactionBuilder::new()
        .sender(sender)
        .expiration(u64::MAX)
        .add_action(action)
        .build_and_sign_only(signer)?;

    let response = client.submit_transaction(tx).await?;
    print_receipt_json(&response.receipt);
    if !response.receipt.is_success() {
        return Err("Token transfer failed".into());
    }
    println!(
        "   Transferred {} units of {} to {}",
        amount / TOKEN_SCALE,
        token,
        to
    );
    Ok(())
}

async fn create_market(
    client: &LightPoolClient,
    signer: &Signer,
    creator: Address,
    base_token: ContractAddress,
    quote_token: ContractAddress,
) -> Result<ContractAddress, Box<dyn std::error::Error>> {
    let params = CreateMarketParams {
        name: "BTC/USDT".into(),
        base_token,
        quote_token,
        min_order_size: 100_000,
        tick_size: TICK_SIZE,
        maker_fee_bps: 10,
        taker_fee_bps: 20,
        allow_market_orders: true,
        state: MarketState::Active,
        limit_order: true,
        side_book_size: SegmentSize::Large,
        creator,
        access: Default::default(),
    };
    let action = ActionBuilder::create_market(params)?;
    let tx = TransactionBuilder::new()
        .sender(creator)
        .expiration(u64::MAX)
        .add_action(action)
        .build_and_sign_only(signer)?;

    let response = client.submit_transaction(tx).await?;
    print_spot_receipt_json(&response.receipt);
    if !response.receipt.is_success() {
        return Err("Market creation failed".into());
    }
    let market_address = extract_market_address_from_events(&response.receipt)
        .ok_or("Failed to extract market address")?;
    println!("   Market: {}", market_address);
    Ok(market_address)
}

async fn print_trader_balances(
    client: &LightPoolClient,
    btc_token_address: ContractAddress,
    usdt_token_address: ContractAddress,
    trader1_address: Address,
    trader2_address: Address,
) {
    for (token_label, token_contract) in [
        ("btc", btc_token_address),
        ("usdt", usdt_token_address),
    ] {
        for (trader_label, account) in [
            ("trader1", trader1_address),
            ("trader2", trader2_address),
        ] {
            let Ok(balance_action) =
                ActionBuilder::get_balance(token_contract, account, GetBalanceParams {})
            else {
                continue;
            };
            let Ok(balance_tx) = TransactionBuilder::new()
                .account(account)
                .expiration(u64::MAX)
                .add_action(balance_action)
                .build_and_without_sign()
            else {
                continue;
            };
            match client.call(balance_tx).await {
                Ok(bytes) => match bincode::deserialize::<GetBalance>(&bytes) {
                    Ok(balance) => println!(
                        "   {} {} - total: {}, locked: {}, available: {}",
                        trader_label,
                        token_label,
                        balance.total / TOKEN_SCALE,
                        balance.locked / TOKEN_SCALE,
                        balance.available / TOKEN_SCALE,
                    ),
                    Err(e) => println!("   ERROR decode {} {}: {}", trader_label, token_label, e),
                },
                Err(e) => println!("   ERROR call {} {}: {}", trader_label, token_label, e),
            }
        }
    }
}
