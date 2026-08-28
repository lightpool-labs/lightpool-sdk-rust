// Copyright (c) LightPool Labs
// Author: xiaoyu1998

//! One isolated margin position → crash mark via `ora_submit` → block-end clearinghouse liquidate.
//!
//! Requires a local node and `~/.lightpool/wallet.json` matching the node validator
//! (same wallet as burst client) so staking / oracle are accepted.
//!
//! ```bash
//! cargo run --release --example simple_clearinghouse_liquidate_client
//! ```
//!
//! Watch the **node** log for:
//! `clearinghouse liquidate start drained=...`

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use env_logger::Env;
use lightpool_sdk::{
    extract_borrowed_from_events, extract_margin_created_from_events, extract_market_address_from_events,
    extract_pool_created_from_events, extract_token_address_from_events, margin_trading_account,
    ActionBuilder, Address, AllocateStakeParams, BondLplParams, BorrowParams, ClearingHouseEvent,
    ContractAddress, CreateMarginParams, CreateMarketParams, CreatePoolParams, CreateTokenParams,
    InitStakingConfigParams, LightPoolClient, MarketState, Message,
    OrderParamsType, OrderSide, PlaceOrderParams, RegisterValidatorParams, SegmentSize, Signer,
    StakePurpose, Subscription, SupplyParams, TimeInForce,
    TransactionBuilder, TransferParams, WebSocketClient, MARGIN_MODE_ISOLATED, TOKEN_SCALE,
};
use log::{info, warn};
use serde_json::json;

const EPOCH_LENGTH: u64 = 1000;
const MIN_BOND: u64 = 10_000 * TOKEN_SCALE;
const BOND_AMOUNT: u64 = 50_000 * TOKEN_SCALE;

const LENDER_SUPPLY: u64 = 1_000_000 * TOKEN_SCALE;
const COLLATERAL: u64 = 20_000 * TOKEN_SCALE;
const BORROW_AMOUNT: u64 = 40_000 * TOKEN_SCALE;
const TRADE_AMOUNT: u64 = 1 * TOKEN_SCALE;
const TRADE_PRICE: u64 = 50_000 * TOKEN_SCALE;
const CRASH_PRICE: u64 = 1_000 * TOKEN_SCALE;
const MIN_ORDER_SIZE: u64 = 100_000;
const TICK_SIZE: u64 = 1_000_000;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Simple isolated position + ora_submit → clearinghouse liquidate"
)]
struct Cli {
    #[clap(long, default_value = "127.0.0.1")]
    address: String,

    /// Skip waiting for first checkpoint (only if running committee already active).
    #[clap(long, default_value_t = false)]
    skip_checkpoint: bool,

    /// Seconds to wait for ClearingHouseEvent::Liquidated after ora_submit.
    #[clap(long, default_value = "60")]
    wait_secs: u64,
}

async fn submit_ok(
    client: &LightPoolClient,
    signer: &Signer,
    sender: Address,
    account: Option<Address>,
    actions: Vec<lightpool_sdk::Action>,
) -> Result<lightpool_sdk::TransactionReceipt, String> {
    let mut tx = TransactionBuilder::new()
        .sender(sender)
        .expiration(u64::MAX);
    if let Some(account) = account {
        tx = tx.account(account);
    }
    for action in actions {
        tx = tx.add_action(action);
    }
    let signed = tx
        .build_and_sign_only(signer)
        .map_err(|e| format!("build tx: {e}"))?;
    let response = client
        .submit_transaction(signed)
        .await
        .map_err(|e| format!("submit tx: {e}"))?;
    if !response.receipt.is_success() {
        return Err(format!("tx failed: {:?}", response.receipt.status));
    }
    Ok(response.receipt)
}

fn load_default_wallet_signer() -> Result<Signer, String> {
    let path = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?
        .join(".lightpool")
        .join("wallet.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("read default wallet {}: {e}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("parse wallet.json: {e}"))?;
    let pk_hex = value
        .get("private_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("wallet {} missing private_key", path.display()))?
        .trim()
        .trim_start_matches("0x");
    let bytes = hex::decode(pk_hex).map_err(|e| format!("wallet private_key hex: {e}"))?;
    if bytes.len() != 32 {
        return Err(format!(
            "wallet private_key must be 32 bytes, got {}",
            bytes.len()
        ));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Signer::from_secret_key_bytes(&key).map_err(|e| format!("Signer from default wallet: {e}"))
}

async fn rpc_committed_tip(rpc: &str) -> Result<u64, String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "getSyncInfo",
        "params": [],
        "id": 1
    });
    let value: serde_json::Value = reqwest::Client::new()
        .post(rpc)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("getSyncInfo http: {e}"))?
        .json()
        .await
        .map_err(|e| format!("getSyncInfo json: {e}"))?;
    if let Some(err) = value.get("error") {
        return Err(format!("getSyncInfo rpc error: {err}"));
    }
    value
        .pointer("/result/committed_block_num")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("getSyncInfo missing committed_block_num: {value}"))
}

async fn create_token(
    client: &LightPoolClient,
    signer: &Signer,
    name: &str,
    symbol: &str,
    supply: u64,
) -> Result<ContractAddress, String> {
    let sender = signer.address();
    let action = ActionBuilder::create_token(CreateTokenParams {
        name: name.into(),
        symbol: symbol.into(),
        total_supply: supply,
        mintable: true,
        to: sender,
    })
    .map_err(|e| format!("create_token: {e}"))?;
    let receipt = submit_ok(client, signer, sender, None, vec![action]).await?;
    extract_token_address_from_events(&receipt)
        .ok_or_else(|| "missing token_created".to_string())
}

async fn transfer(
    client: &LightPoolClient,
    signer: &Signer,
    token: ContractAddress,
    to: Address,
    amount: u64,
) -> Result<(), String> {
    let action = ActionBuilder::transfer_token(token, TransferParams { to, amount })
        .map_err(|e| format!("transfer: {e}"))?;
    submit_ok(client, signer, signer.address(), None, vec![action]).await?;
    Ok(())
}

async fn setup_staking(client: &LightPoolClient, validator: &Signer) -> Result<(), String> {
    info!(
        "Staking (default wallet {}): LPL + init + register + bond + allocate Committee",
        validator.address()
    );
    let lpl = create_token(
        client,
        validator,
        "LightPool",
        "LPL",
        100_000_000 * TOKEN_SCALE,
    )
    .await?;

    let init = ActionBuilder::init_staking_config(InitStakingConfigParams {
        lpl_token: lpl,
        min_bond: MIN_BOND,
        committee_size: 1,
        unbonding_period_blocks: 0,
    })
    .map_err(|e| e.to_string())?;
    match submit_ok(client, validator, validator.address(), None, vec![init]).await {
        Ok(_) => info!("Staking config initialized"),
        Err(e) => warn!("init_staking_config skipped/failed ({e}); continuing"),
    }

    let register = ActionBuilder::register_validator(RegisterValidatorParams {
        consensus_pubkey: *validator.public_key(),
    })
    .map_err(|e| e.to_string())?;
    match submit_ok(client, validator, validator.address(), None, vec![register]).await {
        Ok(_) => info!("Validator registered"),
        Err(e) => warn!("register_validator skipped/failed ({e}); continuing"),
    }

    let bond = ActionBuilder::bond_lpl(BondLplParams {
        lpl_token: lpl,
        amount: BOND_AMOUNT,
    })
    .map_err(|e| e.to_string())?;
    submit_ok(client, validator, validator.address(), None, vec![bond])
        .await
        .map_err(|e| format!("bond_lpl: {e}"))?;
    info!("Bonded {} LPL", BOND_AMOUNT / TOKEN_SCALE);

    let allocate = ActionBuilder::allocate_stake(AllocateStakeParams {
        purpose: StakePurpose::Committee,
        amount: BOND_AMOUNT,
    })
    .map_err(|e| e.to_string())?;
    submit_ok(client, validator, validator.address(), None, vec![allocate])
        .await
        .map_err(|e| format!("allocate_stake: {e}"))?;
    info!("Allocated stake to Committee; running committee after checkpoint prom");
    Ok(())
}

/// Rest tiny GTC sells (above market) so commits keep moving until tip >= target.
async fn drive_tip_to_checkpoint(
    client: &LightPoolClient,
    rpc: &str,
    seller: &Signer,
    market: ContractAddress,
    base: ContractAddress,
    target: u64,
) -> Result<u64, String> {
    let mut tip = rpc_committed_tip(rpc).await.unwrap_or(0);
    if tip >= target {
        info!("Tip already past checkpoint ({tip} >= {target})");
        return Ok(tip);
    }
    info!("Driving tip {tip} → {target} with resting place_order...");
    let rest_price = TRADE_PRICE.saturating_mul(2);
    let deadline = Instant::now() + Duration::from_secs(3600);
    let mut last_log = 0u64;
    while tip < target {
        if Instant::now() >= deadline {
            return Err(format!("timed out waiting for tip >= {target} (last={tip})"));
        }
        let action = ActionBuilder::place_order(
            market,
            PlaceOrderParams {
                side: OrderSide::Sell,
                amount: MIN_ORDER_SIZE,
                order_type: OrderParamsType::Limit {
                    tif: TimeInForce::GTC,
                },
                limit_price: rest_price,
                token_address: base,
            },
        )
        .map_err(|e| e.to_string())?;
        if let Err(e) = submit_ok(client, seller, seller.address(), None, vec![action]).await {
            warn!("checkpoint place_order failed: {e}");
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        tip = rpc_committed_tip(rpc).await.unwrap_or(tip);
        if tip >= last_log + 50 {
            info!("Waiting checkpoint: tip={tip} target={target}");
            last_log = tip;
        }
    }
    info!("Checkpoint reached: tip={tip}");
    Ok(tip)
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let rpc = format!("http://{}:26300", cli.address);
    let ws_url = format!("ws://{}:26400", cli.address);
    info!("Simple clearinghouse liquidate");
    info!("RPC={rpc}");

    let client = LightPoolClient::new(&rpc).with_timeout(Duration::from_secs(60));
    match client.health_check().await {
        Ok(true) => info!("Node healthy"),
        Ok(false) => return Err("Node not healthy".into()),
        Err(e) => return Err(format!("health check: {e}")),
    }

    let lender = load_default_wallet_signer()?;
    let borrower = Signer::new();
    let seller = Signer::new();
    let bidder = Signer::new();
    info!("Lender (default wallet): {}", lender.address());
    info!("Borrower: {}", borrower.address());
    info!("Seller: {}", seller.address());
    info!("Bidder: {}", bidder.address());

    setup_staking(&client, &lender).await?;

    // --- Tokens / pool / market ---
    let usdt = create_token(&client, &lender, "USDT", "USDT", 10_000_000 * TOKEN_SCALE).await?;
    let btc = create_token(&client, &seller, "BTC", "BTC", 21_000_000 * TOKEN_SCALE).await?;
    transfer(&client, &lender, usdt, borrower.address(), COLLATERAL).await?;
    // Bid liquidity for clearinghouse IOC sell of base at crash mark.
    let bid_quote = ((TRADE_AMOUNT as u128) * (CRASH_PRICE as u128) / (TOKEN_SCALE as u128)
        * 2) as u64;
    transfer(&client, &lender, usdt, bidder.address(), bid_quote.max(CRASH_PRICE)).await?;

    let pool_action = ActionBuilder::create_margin_pool(CreatePoolParams {
        token: usdt,
        max_ltv_bps: 8_000,
        maint_bps: 8_500,
        liq_bonus_bps: 500,
    })
    .map_err(|e| e.to_string())?;
    let pool_receipt =
        submit_ok(&client, &lender, lender.address(), None, vec![pool_action]).await?;
    let pool = extract_pool_created_from_events(&pool_receipt)
        .ok_or_else(|| "missing pool_created".to_string())?
        .pool;
    info!("Pool={pool}");

    let supply = ActionBuilder::supply_margin_pool(
        pool,
        SupplyParams {
            amount: LENDER_SUPPLY,
        },
    )
    .map_err(|e| e.to_string())?;
    submit_ok(&client, &lender, lender.address(), None, vec![supply]).await?;

    let market_action = ActionBuilder::create_market(CreateMarketParams {
        name: "BTC/USDT".into(),
        base_token: btc,
        quote_token: usdt,
        min_order_size: MIN_ORDER_SIZE,
        tick_size: TICK_SIZE,
        maker_fee_bps: 10,
        taker_fee_bps: 20,
        allow_market_orders: true,
        state: MarketState::Active,
        limit_order: true,
        side_book_size: SegmentSize::Large,
        creator: lender.address(),
        access: Default::default(),
    })
    .map_err(|e| e.to_string())?;
    let market_receipt =
        submit_ok(&client, &lender, lender.address(), None, vec![market_action]).await?;
    let market = extract_market_address_from_events(&market_receipt)
        .ok_or_else(|| "missing market".to_string())?;
    info!("Market={market}");

    // --- Isolated position: deposit → borrow → buy 1 BTC ---
    let create_margin = ActionBuilder::create_margin_account(CreateMarginParams {
        pool,
        mode: MARGIN_MODE_ISOLATED,
        market: Some(market),
        amount: COLLATERAL,
        margin: None,
    })
    .map_err(|e| e.to_string())?;
    let margin_receipt = submit_ok(
        &client,
        &borrower,
        borrower.address(),
        None,
        vec![create_margin],
    )
    .await?;
    let margin = extract_margin_created_from_events(&margin_receipt)
        .ok_or_else(|| "missing margin_created".to_string())?
        .margin;
    let margin_addr = margin_trading_account(margin);
    info!("Margin={margin} trading={margin_addr}");
    info!(
        "create_margin deposit={} USDT (human)",
        COLLATERAL / TOKEN_SCALE
    );

    info!(
        "Submitting borrow amount={} USDT (human)",
        BORROW_AMOUNT / TOKEN_SCALE
    );
    let borrow = ActionBuilder::borrow_margin(
        margin,
        BorrowParams {
            pool,
            amount: BORROW_AMOUNT,
        },
    )
    .map_err(|e| e.to_string())?;
    let borrow_receipt = submit_ok(
        &client,
        &borrower,
        borrower.address(),
        None,
        vec![borrow],
    )
    .await
    .map_err(|e| format!("borrow failed: {e}"))?;
    match extract_borrowed_from_events(&borrow_receipt) {
        Some(ev) => info!(
            "borrow ok: event amount={} debt={} (human {} / {})",
            ev.amount,
            ev.debt,
            ev.amount / TOKEN_SCALE,
            ev.debt / TOKEN_SCALE
        ),
        None => {
            return Err(
                "borrow tx succeeded but missing margin_borrowed event".into(),
            );
        }
    }

    let sell = ActionBuilder::place_order(
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
    )
    .map_err(|e| e.to_string())?;
    submit_ok(&client, &seller, seller.address(), None, vec![sell]).await?;

    let buy = ActionBuilder::place_order(
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
    )
    .map_err(|e| e.to_string())?;
    submit_ok(
        &client,
        &borrower,
        borrower.address(),
        Some(margin_addr),
        vec![buy],
    )
    .await?;
    info!("Position open: ~1 BTC @ {} with debt {}", TRADE_PRICE / TOKEN_SCALE, BORROW_AMOUNT / TOKEN_SCALE);

    // Bid so clearinghouse can sell base at crash mark.
    let bid = ActionBuilder::place_order(
        market,
        PlaceOrderParams {
            side: OrderSide::Buy,
            amount: TRADE_AMOUNT,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::GTC,
            },
            limit_price: CRASH_PRICE,
            token_address: usdt,
        },
    )
    .map_err(|e| e.to_string())?;
    submit_ok(&client, &bidder, bidder.address(), None, vec![bid]).await?;
    info!("Bid resting at crash price {}", CRASH_PRICE / TOKEN_SCALE);

    // Transfer some BTC to seller for checkpoint IOC spam (already has mint).
    if !cli.skip_checkpoint {
        let tip_now = rpc_committed_tip(&rpc).await.unwrap_or(0);
        let target = ((tip_now / EPOCH_LENGTH) + 1) * EPOCH_LENGTH;
        drive_tip_to_checkpoint(&client, &rpc, &seller, market, btc, target).await?;
    } else {
        warn!(
            "--skip-checkpoint: ora_submit needs running committee; will fail if tip < next epoch boundary"
        );
    }

    // WS logger for ClearingHouseEvent::Liquidated
    let stop = Arc::new(AtomicBool::new(false));
    let liq_count = Arc::new(AtomicU64::new(0));
    let expected_margin = margin;
    let ws_handle = {
        let stop = Arc::clone(&stop);
        let counter = Arc::clone(&liq_count);
        tokio::spawn(async move {
            let mut ws = match WebSocketClient::new(Some(ws_url)).await {
                Ok(c) => c,
                Err(e) => {
                    warn!("ws connect failed: {e}");
                    return;
                }
            };
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            if let Err(e) = ws.subscribe(Subscription::NewBlocks, tx).await {
                warn!("subscribe NewBlocks failed: {e}");
                return;
            }
            while !stop.load(Ordering::Relaxed) {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Some(Message::NewBlock(block) | Message::ReceiptBlock(block)) => {
                                for ev in &block.clearinghouse_events {
                                    let ClearingHouseEvent::Liquidated {
                                        margin,
                                        repay_amount,
                                        seized_amount,
                                        debt,
                                        ..
                                    } = ev;
                                    info!(
                                        "CH Liquidated block={} margin={} repay={} seized={} debt={}",
                                        block.block_num,
                                        margin,
                                        repay_amount,
                                        seized_amount,
                                        debt
                                    );
                                    if *margin == expected_margin {
                                        counter.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                            Some(Message::Error(e)) => {
                                warn!("ws error: {e}");
                                break;
                            }
                            None => break,
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                }
            }
        })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;

    info!(
        "Submitting ora_submit crash mark={} (expect node: clearinghouse liquidate start drained=...)",
        CRASH_PRICE / TOKEN_SCALE
    );
    let oracle = ActionBuilder::submit_oracle_price_for_market(market, CRASH_PRICE)
        .map_err(|e| e.to_string())?;
    match submit_ok(&client, &lender, lender.address(), None, vec![oracle]).await {
        Ok(_) => info!("ora_submit accepted"),
        Err(e) => {
            return Err(format!(
                "ora_submit failed ({e}); need running committee after checkpoint"
            ));
        }
    }

    let deadline = Instant::now() + Duration::from_secs(cli.wait_secs.max(1));
    while liq_count.load(Ordering::Relaxed) == 0 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    stop.store(true, Ordering::Relaxed);
    let _ = ws_handle.await;

    let n = liq_count.load(Ordering::Relaxed);
    if n == 0 {
        warn!(
            "No ClearingHouseEvent for margin {margin} within {}s — check node epilogue / drained log",
            cli.wait_secs
        );
    } else {
        info!("Success: saw {n} clearinghouse liquidation(s) for margin {margin}");
    }
    Ok(())
}
