// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use env_logger::Env;
use lightpool_sdk::lightpool_types::call::{GetBalance, GetBalanceParams};
use lightpool_sdk::{
    ActionBuilder, ContractAddress, CreateEventContractParams, CreateTokenParams,
    LightPoolClient, MintEventContractParams, BurnEventContractParams,
    RedeemEventContractParams, ResolveEventContractParams, Signer, TOKEN_SCALE,
    TransactionBuilder, extract_event_contract_created_from_events,
    extract_token_address_from_events, print_event_contract_receipt_json, print_receipt_json,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MINT_AMOUNT: u64 = 10_000 * TOKEN_SCALE;
const BURN_AMOUNT: u64 = 8_000 * TOKEN_SCALE;
const REDEEM_AMOUNT: u64 = MINT_AMOUNT - BURN_AMOUNT;
const OUTCOME_YES: u8 = 0;
const RESOLUTION_DEADLINE_OFFSET_SECS: u64 = 2;
const RESOLUTION_WAIT_SECS: u64 = RESOLUTION_DEADLINE_OFFSET_SECS + 1;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("LightPool Event Contract Example");
    println!("=================================");

    let trader_signer = Signer::new();
    let trader_address = trader_signer.address();

    println!("Trader address: {}", trader_address);

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
    println!("----------------------------");
    let usdt_create_params = CreateTokenParams {
        name: "USD Tether".into(),
        symbol: "USDT".into(),
        total_supply: 1_000_000 * TOKEN_SCALE,
        mintable: true,
        to: trader_address,
    };
    let usdt_create_action = ActionBuilder::create_token(usdt_create_params)?;
    let usdt_create_tx = TransactionBuilder::new()
        .sender(trader_address)
        .expiration(u64::MAX)
        .add_action(usdt_create_action)
        .build_and_sign_only(&trader_signer)?;

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

    print_balances(&client, trader_address, &[("usdt", usdt_token)]).await;

    println!("\nStep 2: Creating event contract market");
    println!("--------------------------------------");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_secs();
    let create_market_params = CreateEventContractParams {
        question: "Will BTC reach 100k by end of 2026?".to_string(),
        oracle: trader_address,
        collateral_token: usdt_token,
        resolution_deadline: now + RESOLUTION_DEADLINE_OFFSET_SECS,
        tick_size: 10_000,
        min_order_size: 100_000,
        maker_fee_bps: 10,
        taker_fee_bps: 20,
        allow_market_orders: true,
        neg_risk_group_id: None,
    };
    let create_market_action = ActionBuilder::create_event_contract(create_market_params)?;
    let create_market_tx = TransactionBuilder::new()
        .sender(trader_address)
        .expiration(u64::MAX)
        .add_action(create_market_action)
        .build_and_sign_only(&trader_signer)?;

    let market_info = match client.submit_transaction(create_market_tx).await {
        Ok(response) => {
            print_event_contract_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                let created = extract_event_contract_created_from_events(&response.receipt)
                    .expect("Failed to extract event contract created event");
                println!("   Event contract market created: {}", created.market_address);
                println!("   YES token: {}", created.yes_token);
                println!("   NO token: {}", created.no_token);
                created
            } else {
                println!("   ERROR: Event contract market creation failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit market creation transaction: {}", e);
            return Ok(());
        }
    };

    println!("\nStep 3: Minting complete set");
    println!("-----------------------------");
    let mint_params = MintEventContractParams {
        amount: MINT_AMOUNT,
        collateral_token: market_info.collateral_token,
        yes_token: market_info.yes_token,
        no_token: market_info.no_token,
    };
    let mint_action =
        ActionBuilder::mint_event_contract(market_info.market_address, mint_params)?;
    let mint_tx = TransactionBuilder::new()
        .sender(trader_address)
        .expiration(u64::MAX)
        .add_action(mint_action)
        .build_and_sign_only(&trader_signer)?;

    match client.submit_transaction(mint_tx).await {
        Ok(response) => {
            print_event_contract_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   Minted {} complete sets", MINT_AMOUNT / TOKEN_SCALE);
            } else {
                println!("   ERROR: Mint failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit mint transaction: {}", e);
            return Ok(());
        }
    }

    print_balances(
        &client,
        trader_address,
        &[
            ("usdt", market_info.collateral_token),
            ("yes", market_info.yes_token),
            ("no", market_info.no_token),
        ],
    )
    .await;

    println!("\nStep 4: Burning complete set");
    println!("-----------------------------");
    let burn_params = BurnEventContractParams {
        amount: BURN_AMOUNT,
        collateral_token: market_info.collateral_token,
        yes_token: market_info.yes_token,
        no_token: market_info.no_token,
    };
    let burn_action =
        ActionBuilder::burn_event_contract(market_info.market_address, burn_params)?;
    let burn_tx = TransactionBuilder::new()
        .sender(trader_address)
        .expiration(u64::MAX)
        .add_action(burn_action)
        .build_and_sign_only(&trader_signer)?;

    match client.submit_transaction(burn_tx).await {
        Ok(response) => {
            print_event_contract_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   Burned {} complete sets", BURN_AMOUNT / TOKEN_SCALE);
            } else {
                println!("   ERROR: Burn failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit burn transaction: {}", e);
            return Ok(());
        }
    }

    print_balances(
        &client,
        trader_address,
        &[
            ("usdt", market_info.collateral_token),
            ("yes", market_info.yes_token),
            ("no", market_info.no_token),
        ],
    )
    .await;

    println!("\nStep 5: Waiting for resolution deadline");
    println!("----------------------------------------");
    println!(
        "   Sleeping {} seconds until resolution_deadline passes...",
        RESOLUTION_WAIT_SECS
    );
    tokio::time::sleep(Duration::from_secs(RESOLUTION_WAIT_SECS)).await;

    println!("\nStep 6: Resolving event contract (YES wins)");
    println!("--------------------------------------------");
    let resolve_params = ResolveEventContractParams {
        outcome: OUTCOME_YES,
    };
    let resolve_action =
        ActionBuilder::resolve_event_contract(market_info.market_address, resolve_params)?;
    let resolve_tx = TransactionBuilder::new()
        .sender(trader_address)
        .expiration(u64::MAX)
        .add_action(resolve_action)
        .build_and_sign_only(&trader_signer)?;

    match client.submit_transaction(resolve_tx).await {
        Ok(response) => {
            print_event_contract_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!("   Event contract resolved with outcome YES");
            } else {
                println!("   ERROR: Resolve failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit resolve transaction: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 7: Redeeming remaining {} complete sets", REDEEM_AMOUNT / TOKEN_SCALE);
    println!("---------------------------------------------");
    let redeem_params = RedeemEventContractParams {
        collateral_token: market_info.collateral_token,
        yes_token: market_info.yes_token,
        no_token: market_info.no_token,
    };
    let redeem_action =
        ActionBuilder::redeem_event_contract(market_info.market_address, redeem_params)?;
    let redeem_tx = TransactionBuilder::new()
        .sender(trader_address)
        .expiration(u64::MAX)
        .add_action(redeem_action)
        .build_and_sign_only(&trader_signer)?;

    match client.submit_transaction(redeem_tx).await {
        Ok(response) => {
            print_event_contract_receipt_json(&response.receipt);
            if response.receipt.is_success() {
                println!(
                    "   Redeemed {} YES and {} NO for collateral",
                    REDEEM_AMOUNT / TOKEN_SCALE,
                    REDEEM_AMOUNT / TOKEN_SCALE,
                );
            } else {
                println!("   ERROR: Redeem failed!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit redeem transaction: {}", e);
            return Ok(());
        }
    }

    println!("\nStep 8: Querying balances after redeem");
    print_balances(
        &client,
        trader_address,
        &[
            ("usdt", market_info.collateral_token),
            ("yes", market_info.yes_token),
            ("no", market_info.no_token),
        ],
    )
    .await;

    println!("\nEvent contract example completed successfully!");
    Ok(())
}

async fn print_balances(
    client: &LightPoolClient,
    account: lightpool_sdk::Address,
    tokens: &[(&str, ContractAddress)],
) {
    println!("\nQuerying balances via call");
    println!("--------------------------");

    for (label, token_contract) in tokens {
        let balance_action = match ActionBuilder::get_balance(*token_contract, account, GetBalanceParams {}) {
            Ok(action) => action,
            Err(e) => {
                println!("   ERROR: Failed to build {} balance action: {}", label, e);
                continue;
            }
        };

        let balance_tx = match TransactionBuilder::new()
            .account(account)
            .expiration(u64::MAX)
            .add_action(balance_action)
            .build_and_without_sign()
        {
            Ok(tx) => tx,
            Err(e) => {
                println!("   ERROR: Failed to build {} balance call tx: {}", label, e);
                continue;
            }
        };

        match client.call(balance_tx).await {
            Ok(bytes) => match bincode::deserialize::<GetBalance>(&bytes) {
                Ok(balance) => {
                    println!(
                        "   {} balance - total: {}, locked: {}, available: {}",
                        label,
                        balance.total / TOKEN_SCALE,
                        balance.locked / TOKEN_SCALE,
                        balance.available / TOKEN_SCALE,
                    );
                }
                Err(e) => println!("   ERROR: Failed to decode {} balance: {}", label, e),
            },
            Err(e) => println!("   ERROR: {} balance call failed: {}", label, e),
        }
    }
}
