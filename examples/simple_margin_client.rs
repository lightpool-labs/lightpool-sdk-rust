// Copyright (c) LightPool Labs
// Author: xiaoyu1998

//! Margin trading sample: funding pool + isolated margin + borrow + spot buy,
//! then crash BTC mark (oracle + print trade) and liquidate.
//!
//! `cargo run --example simple_margin_client`
//!
//! Oracle submit requires a running-committee validator stake. If that fails,
//! the crash print trade still updates `last_price`, which margin health uses
//! when `mark_price` is unset.

use env_logger::Env;
use lightpool_sdk::lightpool_types::call::{GetBalance, GetBalanceParams};
use lightpool_sdk::{
    ActionBuilder, BorrowParams, ContractAddress, CreateMarginParams, CreateMarketParams,
    CreatePoolParams, CreateTokenParams, DepositCollateralParams, LightPoolClient, LiquidateParams,
    MarketState, OrderParamsType, OrderSide, PlaceOrderParams, SegmentSize, Signer, SupplyParams,
    TOKEN_SCALE, TimeInForce, TransactionBuilder, TransferParams,
    WithdrawSupplyParams, MARGIN_MODE_ISOLATED, extract_borrowed_from_events,
    extract_collateral_deposited_from_events, extract_liquidated_from_events,
    extract_margin_created_from_events, extract_market_address_from_events,
    extract_pool_created_from_events, extract_supplied_from_events,
    extract_supply_withdrawn_from_events, extract_token_address_from_events,
    margin_trading_account, print_margin_receipt_json, print_receipt_json, print_spot_receipt_json,
};
use std::time::Duration;

const LENDER_SUPPLY: u64 = 1_000_000 * TOKEN_SCALE;
/// Sized so after buying 1 BTC @ 50k the account stays healthy, then fails at crash mark.
const COLLATERAL: u64 = 20_000 * TOKEN_SCALE;
const BORROW_AMOUNT: u64 = 40_000 * TOKEN_SCALE;
const TRADE_AMOUNT: u64 = 1 * TOKEN_SCALE;
const TRADE_PRICE: u64 = 50_000 * TOKEN_SCALE;
const CRASH_PRICE: u64 = 1_000 * TOKEN_SCALE;
const CRASH_AMOUNT: u64 = 100_000; // min_order_size

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("LightPool Margin Trading Example (oracle crash + liquidate)");
    println!("===========================================================");

    let lender_signer = Signer::new();
    let lender = lender_signer.address();
    let borrower_signer = Signer::new();
    let borrower = borrower_signer.address();
    let seller_signer = Signer::new();
    let seller = seller_signer.address();

    println!("Lender:   {}", lender);
    println!("Borrower: {}", borrower);
    println!("Seller:   {}", seller);

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

    println!("\nStep 1: Creating USDT token");
    println!("---------------------------");
    let usdt = create_token(
        &client,
        &lender_signer,
        lender,
        "USD Tether",
        "USDT",
        10_000_000 * TOKEN_SCALE,
        lender,
    )
    .await?;

    println!("\nStep 2: Creating BTC token");
    println!("--------------------------");
    let btc = create_token(
        &client,
        &seller_signer,
        seller,
        "Bitcoin",
        "BTC",
        21_000_000 * TOKEN_SCALE,
        seller,
    )
    .await?;

    println!("\nStep 3: Funding borrower with USDT collateral");
    println!("---------------------------------------------");
    transfer_token(
        &client,
        &lender_signer,
        lender,
        usdt,
        borrower,
        COLLATERAL,
    )
    .await?;

    println!("\nStep 4: Creating USDT funding pool");
    println!("----------------------------------");
    let create_pool_action = ActionBuilder::create_margin_pool(CreatePoolParams {
        token: usdt,
        max_ltv_bps: 8_000,
        maint_bps: 8_500,
        liq_bonus_bps: 500,
    })?;
    let create_pool_tx = TransactionBuilder::new()
        .sender(lender)
        .expiration(u64::MAX)
        .add_action(create_pool_action)
        .build_and_sign_only(&lender_signer)?;

    let pool = match client.submit_transaction(create_pool_tx).await {
        Ok(response) => {
            print_margin_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let created = extract_pool_created_from_events(&response.receipt)
                    .expect("missing event margin_pool_created");
                println!(
                    "   event margin_pool_created: pool={}, token={}, max_ltv_bps={}, maint_bps={}",
                    created.pool, created.token, created.max_ltv_bps, created.maint_bps
                );
                created.pool
            } else {
                println!("   ERROR: Pool creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit create pool tx: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 5: Lender supplies USDT to pool");
    println!("------------------------------------");
    let supply_action = ActionBuilder::supply_margin_pool(
        pool,
        SupplyParams {
            amount: LENDER_SUPPLY,
        },
    )?;
    let supply_tx = TransactionBuilder::new()
        .sender(lender)
        .expiration(u64::MAX)
        .add_action(supply_action)
        .build_and_sign_only(&lender_signer)?;

    match client.submit_transaction(supply_tx).await {
        Ok(response) => {
            print_margin_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let supplied = extract_supplied_from_events(&response.receipt)
                    .expect("missing event margin_supplied");
                println!(
                    "   event margin_supplied: lender={}, amount={}, shares={}",
                    supplied.lender,
                    supplied.amount / TOKEN_SCALE,
                    supplied.shares / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Supply failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit supply tx: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 6: Creating BTC/USDT market");
    println!("--------------------------------");
    let market_create_action = ActionBuilder::create_market(CreateMarketParams {
        name: "BTC/USDT".into(),
        base_token: btc,
        quote_token: usdt,
        min_order_size: CRASH_AMOUNT,
        tick_size: 1_000_000,
        maker_fee_bps: 10,
        taker_fee_bps: 20,
        allow_market_orders: true,
        state: MarketState::Active,
        limit_order: true,
        side_book_size: SegmentSize::Large,
        creator: lender,
        access: Default::default(),
    })?;
    let market_create_tx = TransactionBuilder::new()
        .sender(lender)
        .expiration(u64::MAX)
        .add_action(market_create_action)
        .build_and_sign_only(&lender_signer)?;

    let market = match client.submit_transaction(market_create_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let market = extract_market_address_from_events(&response.receipt)
                    .expect("Failed to extract market");
                println!("   Market: {}", market);
                market
            } else {
                println!("   ERROR: Market creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit market create tx: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 7: Creating isolated margin account");
    println!("----------------------------------------");
    let create_margin_action = ActionBuilder::create_margin_account(CreateMarginParams {
        pool,
        mode: MARGIN_MODE_ISOLATED,
        market: Some(market),
        amount: COLLATERAL,
        margin: None,
    })?;
    let create_margin_tx = TransactionBuilder::new()
        .sender(borrower)
        .expiration(u64::MAX)
        .add_action(create_margin_action)
        .build_and_sign_only(&borrower_signer)?;

    let (margin, margin_addr) = match client.submit_transaction(create_margin_tx).await {
        Ok(response) => {
            print_margin_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let created = extract_margin_created_from_events(&response.receipt)
                    .expect("missing event margin_account_created");
                let trading = margin_trading_account(created.margin);
                println!(
                    "   event margin_account_created: margin={}, pool={}, owner={}, mode={}, market={:?}, amount={}",
                    created.margin, created.pool, created.owner, created.mode, created.market, created.amount
                );
                println!("   Trading account: {}", trading);
                (created.margin, trading)
            } else {
                println!("   ERROR: Create margin failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit create margin tx: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 8: Depositing collateral into margin account");
    println!("-------------------------------------------------");
    let deposit_action = ActionBuilder::deposit_margin_collateral(
        margin,
        DepositCollateralParams {
            amount: COLLATERAL,
        },
    )?;
    let deposit_tx = TransactionBuilder::new()
        .sender(borrower)
        .expiration(u64::MAX)
        .add_action(deposit_action)
        .build_and_sign_only(&borrower_signer)?;

    match client.submit_transaction(deposit_tx).await {
        Ok(response) => {
            print_margin_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let deposited = extract_collateral_deposited_from_events(&response.receipt)
                    .expect("missing event margin_collateral_deposited");
                println!(
                    "   event margin_collateral_deposited: user={}, amount={}",
                    deposited.user,
                    deposited.amount / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Deposit collateral failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit deposit tx: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 9: Borrowing USDT from pool");
    println!("--------------------------------");
    let borrow_action = ActionBuilder::borrow_margin(
        margin,
        BorrowParams {
            pool,
            amount: BORROW_AMOUNT,
        },
    )?;
    let borrow_tx = TransactionBuilder::new()
        .sender(borrower)
        .expiration(u64::MAX)
        .add_action(borrow_action)
        .build_and_sign_only(&borrower_signer)?;

    match client.submit_transaction(borrow_tx).await {
        Ok(response) => {
            print_margin_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let borrowed = extract_borrowed_from_events(&response.receipt)
                    .expect("missing event margin_borrowed");
                println!(
                    "   event margin_borrowed: amount={}, debt={}",
                    borrowed.amount / TOKEN_SCALE,
                    borrowed.debt / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Borrow failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit borrow tx: {}", e);
            return Ok(());
        }
    }

    print_balance(&client, "margin USDT", usdt, margin_addr).await;

    println!("\nStep 10: Seller places resting sell");
    println!("-----------------------------------");
    let sell_action = ActionBuilder::place_order(
        market,
        PlaceOrderParams {
            side: OrderSide::Sell,
            amount: TRADE_AMOUNT,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::GTC,
            },
            limit_price: TRADE_PRICE,
            token_address: btc,
        },
    )?;
    let sell_tx = TransactionBuilder::new()
        .sender(seller)
        .expiration(u64::MAX)
        .add_action(sell_action)
        .build_and_sign_only(&seller_signer)?;
    match client.submit_transaction(sell_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if !response.receipt.is_success() {
                println!("   ERROR: Sell order failed!");
                return Ok(());
            }
            println!("   Seller listed {} BTC", TRADE_AMOUNT / TOKEN_SCALE);
        }
        Err(e) => {
            println!("   ERROR: Failed to submit sell tx: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 11: Margin account buys BTC (borrower as agent)");
    println!("----------------------------------------------------");
    let buy_action = ActionBuilder::place_order(
        market,
        PlaceOrderParams {
            side: OrderSide::Buy,
            amount: TRADE_AMOUNT,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::IOC,
            },
            limit_price: TRADE_PRICE,
            token_address: usdt,
        },
    )?;
    let buy_tx = TransactionBuilder::new()
        .sender(borrower)
        .account(margin_addr)
        .expiration(u64::MAX)
        .add_action(buy_action)
        .build_and_sign_only(&borrower_signer)?;
    match client.submit_transaction(buy_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!(
                    "   Margin bought {} BTC at {} USDT",
                    TRADE_AMOUNT / TOKEN_SCALE,
                    TRADE_PRICE / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Margin buy failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit margin buy tx: {}", e);
            return Ok(());
        }
    }

    print_balance(&client, "margin USDT", usdt, margin_addr).await;
    print_balance(&client, "margin BTC", btc, margin_addr).await;

    println!("\nStep 12: Crash BTC mark via spot oracle");
    println!("---------------------------------------");
    println!("   Submit may fail if sender is not a running-committee validator.");
    let oracle_action = ActionBuilder::submit_oracle_price_for_market(market, CRASH_PRICE)?;
    let oracle_tx = TransactionBuilder::new()
        .sender(lender)
        .expiration(u64::MAX)
        .add_action(oracle_action)
        .build_and_sign_only(&lender_signer)?;
    match client.submit_transaction(oracle_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!(
                    "   Oracle quote submitted at {} USDT (finalize at block end)",
                    CRASH_PRICE / TOKEN_SCALE
                );
            } else {
                println!(
                    "   Oracle submit rejected; continuing with crash print trade for last_price"
                );
            }
        }
        Err(e) => {
            println!(
                "   Oracle submit failed ({}); continuing with crash print trade",
                e
            );
        }
    }

    println!("\nStep 13: Crash print trade to refresh last_price");
    println!("------------------------------------------------");
    let crash_sell_action = ActionBuilder::place_order(
        market,
        PlaceOrderParams {
            side: OrderSide::Sell,
            amount: CRASH_AMOUNT,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::GTC,
            },
            limit_price: CRASH_PRICE,
            token_address: btc,
        },
    )?;
    let crash_sell_tx = TransactionBuilder::new()
        .sender(seller)
        .expiration(u64::MAX)
        .add_action(crash_sell_action)
        .build_and_sign_only(&seller_signer)?;
    match client.submit_transaction(crash_sell_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if !response.receipt.is_success() {
                println!("   ERROR: Crash sell failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit crash sell: {}", e);
            return Ok(());
        }
    }

    let crash_buy_action = ActionBuilder::place_order(
        market,
        PlaceOrderParams {
            side: OrderSide::Buy,
            amount: CRASH_AMOUNT,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::IOC,
            },
            limit_price: CRASH_PRICE,
            token_address: usdt,
        },
    )?;
    let crash_buy_tx = TransactionBuilder::new()
        .sender(lender)
        .expiration(u64::MAX)
        .add_action(crash_buy_action)
        .build_and_sign_only(&lender_signer)?;
    match client.submit_transaction(crash_buy_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!(
                    "   Crash trade filled at {} USDT (last_price for margin mark)",
                    CRASH_PRICE / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Crash buy failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit crash buy: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 14: Lender liquidates under-collateralized margin");
    println!("-----------------------------------------------------");
    let liqd_action = ActionBuilder::liquidate_margin(
        margin,
        LiquidateParams {
            pool,
            repay_amount: BORROW_AMOUNT,
        },
    )?;
    let liqd_tx = TransactionBuilder::new()
        .sender(lender)
        .expiration(u64::MAX)
        .add_action(liqd_action)
        .build_and_sign_only(&lender_signer)?;
    match client.submit_transaction(liqd_tx).await {
        Ok(response) => {
            print_margin_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let ev = extract_liquidated_from_events(&response.receipt)
                    .expect("missing event margin_liquidated");
                println!(
                    "   event margin_liquidated: liquidator={}, repay={}, seized_quote={}, debt_left={}",
                    ev.liquidator,
                    ev.repay_amount / TOKEN_SCALE,
                    ev.seized_amount / TOKEN_SCALE,
                    ev.debt / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Liquidation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit liquidate tx: {}", e);
            return Ok(());
        }
    }

    print_balance(&client, "margin USDT after liqd", usdt, margin_addr).await;
    print_balance(&client, "margin BTC after liqd", btc, margin_addr).await;
    print_balance(&client, "lender USDT", usdt, lender).await;
    print_balance(&client, "lender BTC", btc, lender).await;

    println!("\nStep 15: Lender withdraws part of supply");
    println!("----------------------------------------");
    let wd_sup_action = ActionBuilder::withdraw_margin_supply(
        pool,
        WithdrawSupplyParams {
            shares: LENDER_SUPPLY / 10,
        },
    )?;
    let wd_sup_tx = TransactionBuilder::new()
        .sender(lender)
        .expiration(u64::MAX)
        .add_action(wd_sup_action)
        .build_and_sign_only(&lender_signer)?;
    match client.submit_transaction(wd_sup_tx).await {
        Ok(response) => {
            print_margin_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let withdrawn = extract_supply_withdrawn_from_events(&response.receipt)
                    .expect("missing event margin_supply_withdrawn");
                println!(
                    "   event margin_supply_withdrawn: lender={}, shares={}, amount={}",
                    withdrawn.lender,
                    withdrawn.shares / TOKEN_SCALE,
                    withdrawn.amount / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Withdraw supply failed!");
            }
        }
        Err(e) => println!("   ERROR: Failed to submit withdraw supply tx: {}", e),
    }

    println!("\nMargin trading example completed!");
    println!("==================================");
    println!("Summary:");
    println!("1. Created USDT + BTC and funding pool");
    println!("2. Opened isolated margin, deposited, borrowed, bought BTC");
    println!("3. Crashed mark via oracle submit + print trade");
    println!("4. Liquidated the margin account");
    println!("5. Lender withdrew part of supply");

    Ok(())
}

async fn create_token(
    client: &LightPoolClient,
    signer: &Signer,
    sender: lightpool_sdk::Address,
    name: &str,
    symbol: &str,
    total_supply: u64,
    to: lightpool_sdk::Address,
) -> Result<ContractAddress, Box<dyn std::error::Error>> {
    let action = ActionBuilder::create_token(CreateTokenParams {
        name: name.into(),
        symbol: symbol.into(),
        total_supply,
        mintable: true,
        to,
    })?;
    let tx = TransactionBuilder::new()
        .sender(sender)
        .expiration(u64::MAX)
        .add_action(action)
        .build_and_sign_only(signer)?;
    let response = client.submit_transaction(tx).await?;
    print_receipt_json(&response.receipt);
    if !response.receipt.is_success() {
        return Err(format!("{} token creation failed", symbol).into());
    }
    let token = extract_token_address_from_events(&response.receipt)
        .ok_or_else(|| format!("Failed to extract {} token", symbol))?;
    println!("   {} token: {}", symbol, token);
    Ok(token)
}

async fn transfer_token(
    client: &LightPoolClient,
    signer: &Signer,
    sender: lightpool_sdk::Address,
    token: ContractAddress,
    to: lightpool_sdk::Address,
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
    println!("   Transferred {} to {}", amount / TOKEN_SCALE, to);
    Ok(())
}

async fn print_balance(
    client: &LightPoolClient,
    label: &str,
    token: ContractAddress,
    account: lightpool_sdk::Address,
) {
    let Ok(action) = ActionBuilder::get_balance(token, account, GetBalanceParams {}) else {
        println!("   ERROR: Failed to build {} balance action", label);
        return;
    };
    let Ok(tx) = TransactionBuilder::new()
        .account(account)
        .expiration(u64::MAX)
        .add_action(action)
        .build_and_without_sign()
    else {
        println!("   ERROR: Failed to build {} balance call tx", label);
        return;
    };
    match client.call(tx).await {
        Ok(bytes) => match bincode::deserialize::<GetBalance>(&bytes) {
            Ok(balance) => println!(
                "   {} - total: {}, locked: {}, available: {}",
                label,
                balance.total / TOKEN_SCALE,
                balance.locked / TOKEN_SCALE,
                balance.available / TOKEN_SCALE,
            ),
            Err(e) => println!("   ERROR: decode {} balance: {}", label, e),
        },
        Err(e) => println!("   ERROR: {} balance call failed: {}", label, e),
    }
}
