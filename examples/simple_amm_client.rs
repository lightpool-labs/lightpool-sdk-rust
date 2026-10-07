// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use std::time::Duration;

use env_logger::Env;
use lightpool_sdk::{
    extract_amm_pool_from_events, print_receipt_json, ActionBuilder, Address,
    AmmAddLiquidityParams, AmmCreatePoolParams, AmmRemoveLiquidityParams, AmmSwapParams,
    ContractAddress, CreateTokenParams, ExecutionStatus, LightPoolClient, Signer, TOKEN_SCALE,
    TransactionBuilder, TransactionReceipt,
};

const SQRT_PRICE_X96: u128 = 1u128 << 96;
const TICK_SPACING: i32 = 10;
const TICK_LOWER: i32 = -100;
const TICK_UPPER: i32 = 100;
const LIQUIDITY: u128 = 1_000_000;
const SWAP_AMOUNT: u64 = 1_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("LightPool AMM Client Example");
    println!("============================");

    let signer = Signer::new();
    let sender = signer.address();
    println!("Signer: {}", sender);

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

    println!("\nStep 1: Creating two tokens");
    println!("---------------------------");
    let token_a = create_token(&client, &signer, sender, "Token A", "TKA").await?;
    let token_b = create_token(&client, &signer, sender, "Token B", "TKB").await?;
    let (token0, token1) = if token_a < token_b {
        (token_a, token_b)
    } else {
        (token_b, token_a)
    };
    println!("   token0: {}", token0);
    println!("   token1: {}", token1);

    println!("\nStep 2: Creating a pool");
    println!("-----------------------");
    let create_action = ActionBuilder::create_amm_pool(AmmCreatePoolParams {
        whitelist: None,
        token0,
        token1,
        fee: 3_000,
        tick_spacing: TICK_SPACING,
        sqrt_price_x96: SQRT_PRICE_X96,
    })?;
    let create_receipt = submit(&client, &signer, sender, create_action).await?;
    let pool = extract_amm_pool_from_events(&create_receipt).expect("pool_created event");
    println!("   Pool: {}", pool);

    println!("\nStep 3: Adding liquidity");
    println!("------------------------");
    let add_action = ActionBuilder::add_amm_liquidity(
        pool,
        AmmAddLiquidityParams {
            whitelist: None,
            token0,
            token1,
            tick_lower: TICK_LOWER,
            tick_upper: TICK_UPPER,
            liquidity: LIQUIDITY,
        },
    )?;
    submit(&client, &signer, sender, add_action).await?;

    println!("\nStep 4: Swapping token0 for token1");
    println!("----------------------------------");
    let swap_action = ActionBuilder::swap_amm(
        pool,
        AmmSwapParams {
            whitelist: None,
            token0,
            token1,
            zero_for_one: true,
            amount: SWAP_AMOUNT,
            sqrt_price_limit_x96: 0,
        },
    )?;
    submit(&client, &signer, sender, swap_action).await?;

    println!("\nStep 5: Removing liquidity");
    println!("--------------------------");
    let remove_action = ActionBuilder::remove_amm_liquidity(
        pool,
        AmmRemoveLiquidityParams {
            whitelist: None,
            token0,
            token1,
            tick_lower: TICK_LOWER,
            tick_upper: TICK_UPPER,
            liquidity: LIQUIDITY,
        },
    )?;
    submit(&client, &signer, sender, remove_action).await?;

    Ok(())
}

async fn create_token(
    client: &LightPoolClient,
    signer: &Signer,
    sender: Address,
    name: &str,
    symbol: &str,
) -> Result<ContractAddress, Box<dyn std::error::Error>> {
    let action = ActionBuilder::create_token(CreateTokenParams {
        name: name.into(),
        symbol: symbol.into(),
        total_supply: 1_000_000 * TOKEN_SCALE,
        mintable: true,
        to: sender,
    })?;
    let receipt = submit(client, signer, sender, action).await?;
    Ok(lightpool_sdk::extract_token_address_from_events(&receipt).expect("token_created event"))
}

async fn submit(
    client: &LightPoolClient,
    signer: &Signer,
    sender: Address,
    action: lightpool_sdk::Action,
) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
    let tx = TransactionBuilder::new()
        .sender(sender)
        .expiration(u64::MAX)
        .add_action(action)
        .build_and_sign_only(signer)?;
    let response = client.submit_transaction(tx).await?;
    print_receipt_json(&response.receipt);
    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(error_msg.clone().into());
        }
        return Err("transaction failed".into());
    }
    Ok(response.receipt)
}
