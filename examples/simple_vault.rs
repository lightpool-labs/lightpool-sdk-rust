// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use env_logger::Env;
use lightpool_sdk::lightpool_types::call::{GetBalance, GetBalanceParams};
use lightpool_sdk::{
    ActionBuilder, ContractAddress, CreateMarketParams, CreateTokenParams, CreateVaultParams,
    DepositVaultParams, LightPoolClient, MarketState, OrderParamsType, OrderSide, PlaceOrderParams,
    SegmentSize, Signer, TOKEN_SCALE, TimeInForce, TransactionBuilder, TransferParams,
    WithdrawVaultParams, extract_market_address_from_events, extract_token_address_from_events,
    extract_vault_created_from_events, print_receipt_json, print_spot_receipt_json,
    print_vault_receipt_json, vault_account,
};
use std::time::Duration;

const SEED_AMOUNT: u64 = 500_000 * TOKEN_SCALE;
const DEPOSIT_AMOUNT: u64 = 100_000 * TOKEN_SCALE;
const TRADE_AMOUNT: u64 = 2 * TOKEN_SCALE;
const TRADE_PRICE: u64 = 50_000 * TOKEN_SCALE;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("LightPool Vault + Spot Trading Example");
    println!("======================================");

    let manager_signer = Signer::new();
    let manager_address = manager_signer.address();
    let seller_signer = Signer::new();
    let seller_address = seller_signer.address();
    let depositor_signer = Signer::new();
    let depositor_address = depositor_signer.address();

    println!("Manager address:   {}", manager_address);
    println!("Seller address:    {}", seller_address);
    println!("Depositor address: {}", depositor_address);

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

    println!("\nStep 1: Creating USDT token (manager)");
    println!("-------------------------------------");
    let usdt_create_params = CreateTokenParams {
        name: "USD Tether".into(),
        symbol: "USDT".into(),
        total_supply: 10_000_000 * TOKEN_SCALE,
        mintable: true,
        to: manager_address,
    };
    let usdt_create_action = ActionBuilder::create_token(usdt_create_params)?;
    let usdt_create_tx = TransactionBuilder::new()
        .sender(manager_address)
        .expiration(u64::MAX)
        .add_action(usdt_create_action)
        .build_and_sign_only(&manager_signer)?;

    let usdt_token = match client.submit_transaction(usdt_create_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let token = extract_token_address_from_events(&response.receipt)
                    .expect("Failed to extract USDT token contract");
                println!("   USDT token created: {}", token);
                token
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

    println!("\nStep 2: Creating BTC token (seller)");
    println!("-----------------------------------");
    let btc_create_params = CreateTokenParams {
        name: "Bitcoin".into(),
        symbol: "BTC".into(),
        total_supply: 21_000_000 * TOKEN_SCALE,
        mintable: true,
        to: seller_address,
    };
    let btc_create_action = ActionBuilder::create_token(btc_create_params)?;
    let btc_create_tx = TransactionBuilder::new()
        .sender(seller_address)
        .expiration(u64::MAX)
        .add_action(btc_create_action)
        .build_and_sign_only(&seller_signer)?;

    let btc_token = match client.submit_transaction(btc_create_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let token = extract_token_address_from_events(&response.receipt)
                    .expect("Failed to extract BTC token contract");
                println!("   BTC token created: {}", token);
                token
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

    println!("\nStep 3: Funding depositor with USDT");
    println!("-----------------------------------");
    let transfer_params = TransferParams {
        to: depositor_address,
        amount: DEPOSIT_AMOUNT,
    };
    let transfer_action = ActionBuilder::transfer_token(usdt_token, transfer_params)?;
    let transfer_tx = TransactionBuilder::new()
        .sender(manager_address)
        .expiration(u64::MAX)
        .add_action(transfer_action)
        .build_and_sign_only(&manager_signer)?;

    match client.submit_transaction(transfer_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!(
                    "   Transferred {} USDT to depositor",
                    DEPOSIT_AMOUNT / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: USDT transfer to depositor failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit transfer transaction: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 4: Creating vault (manager seed deposit)");
    println!("---------------------------------------------");
    let create_vault_params = CreateVaultParams {
        name: "Demo Vault".into(),
        quote_token: usdt_token,
        share_name: "Vault Share".into(),
        share_symbol: "vUSDT".into(),
        seed_amount: SEED_AMOUNT,
    };
    let create_vault_action = ActionBuilder::create_vault(create_vault_params)?;
    let create_vault_tx = TransactionBuilder::new()
        .sender(manager_address)
        .expiration(u64::MAX)
        .add_action(create_vault_action)
        .build_and_sign_only(&manager_signer)?;

    let vault_info = match client.submit_transaction(create_vault_tx).await {
        Ok(response) => {
            print_vault_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let created = extract_vault_created_from_events(&response.receipt)
                    .expect("Failed to extract vault_created event");
                println!("   Vault created: {}", created.vault);
                println!("   Share token: {}", created.share_token);
                created
            } else {
                println!("   ERROR: Vault creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit vault creation transaction: {}", e);
            return Ok(());
        }
    };

    let vault_addr = vault_account(vault_info.vault);
    println!("   Vault trading account: {}", vault_addr);

    println!("\nStep 5: Depositor deposits into vault");
    println!("-------------------------------------");
    let deposit_params = DepositVaultParams {
        amount: DEPOSIT_AMOUNT,
        quote_token: vault_info.quote_token,
        share_token: vault_info.share_token,
    };
    let deposit_action = ActionBuilder::deposit_vault(vault_info.vault, deposit_params)?;
    let deposit_tx = TransactionBuilder::new()
        .sender(depositor_address)
        .expiration(u64::MAX)
        .add_action(deposit_action)
        .build_and_sign_only(&depositor_signer)?;

    match client.submit_transaction(deposit_tx).await {
        Ok(response) => {
            print_vault_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!(
                    "   Depositor deposited {} USDT",
                    DEPOSIT_AMOUNT / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Vault deposit failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit deposit transaction: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 6: Creating BTC/USDT spot market");
    println!("-------------------------------------");
    let market_create_params = CreateMarketParams {
        name: "BTC/USDT".into(),
        base_token: btc_token,
        quote_token: usdt_token,
        min_order_size: 100_000,
        tick_size: 1_000_000,
        maker_fee_bps: 10,
        taker_fee_bps: 20,
        allow_market_orders: true,
        state: MarketState::Active,
        limit_order: true,
        side_book_size: SegmentSize::Large,
        creator: manager_address,
    };
    let market_create_action = ActionBuilder::create_market(market_create_params)?;
    let market_create_tx = TransactionBuilder::new()
        .sender(manager_address)
        .expiration(u64::MAX)
        .add_action(market_create_action)
        .build_and_sign_only(&manager_signer)?;

    let market_address = match client.submit_transaction(market_create_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let market = extract_market_address_from_events(&response.receipt)
                    .expect("Failed to extract market address");
                println!("   BTC/USDT market created: {}", market);
                market
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

    println!("\nStep 7: Seller places resting sell order");
    println!("----------------------------------------");
    let sell_order_params = PlaceOrderParams {
        side: OrderSide::Sell,
        amount: TRADE_AMOUNT,
        order_type: OrderParamsType::Limit {
            tif: TimeInForce::GTC,
        },
        limit_price: TRADE_PRICE,
        token_address: btc_token,
    };
    let sell_order_action = ActionBuilder::place_order(market_address, sell_order_params)?;
    let sell_order_tx = TransactionBuilder::new()
        .sender(seller_address)
        .expiration(u64::MAX)
        .add_action(sell_order_action)
        .build_and_sign_only(&seller_signer)?;

    match client.submit_transaction(sell_order_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!(
                    "   Seller listed {} BTC at {} USDT",
                    TRADE_AMOUNT / TOKEN_SCALE,
                    TRADE_PRICE / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Sell order placement failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit sell order transaction: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 8: Vault buys BTC (manager as vault agent)");
    println!("------------------------------------------------");
    let buy_order_params = PlaceOrderParams {
        side: OrderSide::Buy,
        amount: TRADE_AMOUNT,
        order_type: OrderParamsType::Limit {
            tif: TimeInForce::IOC,
        },
        limit_price: TRADE_PRICE,
        token_address: usdt_token,
    };
    let buy_order_action = ActionBuilder::place_order(market_address, buy_order_params)?;
    let buy_order_tx = TransactionBuilder::new()
        .sender(manager_address)
        .account(vault_addr)
        .expiration(u64::MAX)
        .add_action(buy_order_action)
        .build_and_sign_only(&manager_signer)?;

    match client.submit_transaction(buy_order_tx).await {
        Ok(response) => {
            print_spot_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!(
                    "   Vault bought {} BTC at {} USDT",
                    TRADE_AMOUNT / TOKEN_SCALE,
                    TRADE_PRICE / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Vault buy order failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit vault buy transaction: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 9: Depositor withdraws half of shares");
    println!("------------------------------------------");
    let withdraw_shares = DEPOSIT_AMOUNT / 2;
    let withdraw_params = WithdrawVaultParams {
        shares: withdraw_shares,
        quote_token: vault_info.quote_token,
        share_token: vault_info.share_token,
    };
    let withdraw_action = ActionBuilder::withdraw_vault(vault_info.vault, withdraw_params)?;
    let withdraw_tx = TransactionBuilder::new()
        .sender(depositor_address)
        .expiration(u64::MAX)
        .add_action(withdraw_action)
        .build_and_sign_only(&depositor_signer)?;

    match client.submit_transaction(withdraw_tx).await {
        Ok(response) => {
            print_vault_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!(
                    "   Depositor withdrew {} shares",
                    withdraw_shares / TOKEN_SCALE
                );
            } else {
                println!("   ERROR: Vault withdraw failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit withdraw transaction: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 10: Querying balances");
    print_balances(
        &client,
        &[
            ("manager", manager_address),
            ("seller", seller_address),
            ("depositor", depositor_address),
            ("vault", vault_addr),
        ],
        &[
            ("usdt", usdt_token),
            ("btc", btc_token),
            ("share", vault_info.share_token),
        ],
    )
    .await;

    println!("\nVault + spot trading example completed successfully!");
    Ok(())
}

async fn print_balances(
    client: &LightPoolClient,
    accounts: &[(&str, lightpool_sdk::Address)],
    tokens: &[(&str, ContractAddress)],
) {
    println!("\nQuerying balances via call");
    println!("--------------------------");

    for (account_label, account) in accounts {
        for (token_label, token_contract) in tokens {
            let balance_action =
                match ActionBuilder::get_balance(*token_contract, *account, GetBalanceParams {}) {
                    Ok(action) => action,
                    Err(e) => {
                        println!(
                            "   ERROR: Failed to build {} {} balance action: {}",
                            account_label, token_label, e
                        );
                        continue;
                    }
                };

            let balance_tx = match TransactionBuilder::new()
                .account(*account)
                .expiration(u64::MAX)
                .add_action(balance_action)
                .build_and_without_sign()
            {
                Ok(tx) => tx,
                Err(e) => {
                    println!(
                        "   ERROR: Failed to build {} {} balance call tx: {}",
                        account_label, token_label, e
                    );
                    continue;
                }
            };

            match client.call(balance_tx).await {
                Ok(bytes) => match bincode::deserialize::<GetBalance>(&bytes) {
                    Ok(balance) => {
                        println!(
                            "   {} {} - total: {}, locked: {}, available: {}",
                            account_label,
                            token_label,
                            balance.total / TOKEN_SCALE,
                            balance.locked / TOKEN_SCALE,
                            balance.available / TOKEN_SCALE,
                        );
                    }
                    Err(e) => println!(
                        "   ERROR: Failed to decode {} {} balance: {}",
                        account_label, token_label, e
                    ),
                },
                Err(e) => println!(
                    "   ERROR: {} {} balance call failed: {}",
                    account_label, token_label, e
                ),
            }
        }
    }
}
