// Copyright (c) LightPool Labs
// Author: xiaoyu1998

//! Burst client for Isolated margin liquidations that sell base via IOC to repay debt.
//!
//! ```bash
//! cargo run --release --example burst_clearinghouse_liquidate_client -- \
//!   --positions 200 --total-positions 200000 \
//!   --num-markets 500 --senders 1024 --rate-per-task 500 --duration 10
//! ```
//!
//! ## Phases (sequential)
//! 1. Promote committee (staking allocate)
//! 2. Tokens / margin pools
//! 3. Spot markets + fund senders
//! 4. Mempool-burst setup isolated positions
//! 5. Pass first checkpoint only if tip is still below it (skip burst if already past)
//! 6. `ora_submit` first (quiet mempool), then spot place_order load (crash → liquidate, or `--no-liquidate`)
//!
//! One mempool TCP connection is shared across all phases via an async channel.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use clap::Parser;
use env_logger::Env;
use futures::SinkExt;
use lightpool_sdk::{
    extract_margin_created_from_events, extract_market_address_from_events,
    extract_pool_created_from_events, extract_token_address_from_events, margin_trading_account,
    ActionBuilder, Address, AllocateStakeParams, BondLplParams, BorrowParams, ClearingHouseEvent,
    ContractAddress, CreateMarginParams, CreateMarketParams, CreatePoolParams, CreateTokenParams,
    DepositCollateralParams, InitStakingConfigParams, LightPoolClient, MarketState, Message,
    OrderParamsType, OrderSide, PlaceOrderParams, RegisterValidatorParams, SegmentSize, Signer,
    StakePurpose, SubmitOraclePriceParams, Subscription, SupplyParams, TimeInForce,
    TransactionBuilder, TransferParams, WebSocketClient, MARGIN_MODE_ISOLATED, TOKEN_SCALE,
};
use lightpool_sdk::lightpool_types::{
    margin_account_contract, market_contract, pool_contract, token_contract,
};
use log::{error, info, warn};
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

const TICK_SIZE: u64 = 1_000_000;
const SPOT_TICK_SIZE: u64 = 100_000;
const MIN_ORDER_SIZE: u64 = 100_000;

/// Matches on-chain epoch length (checkpoint / prom_running boundary).
const EPOCH_LENGTH: u64 = 1000;

const MIN_BOND: u64 = 10_000 * TOKEN_SCALE;
const BOND_AMOUNT: u64 = 50_000 * TOKEN_SCALE;

/// Healthy entry mark / trade price.
const TRADE_PRICE: u64 = 50_000 * TOKEN_SCALE;
/// Crash mark used for oracle / liquidation bids.
const CRASH_PRICE: u64 = 1_000 * TOKEN_SCALE;
const TRADE_AMOUNT: u64 = 1 * TOKEN_SCALE;
const COLLATERAL: u64 = 20_000 * TOKEN_SCALE;
const BORROW_AMOUNT: u64 = 40_000 * TOKEN_SCALE;

const POOLS: usize = 100;
const BORROWERS: usize = 10_000;
/// Isolated-position markets (round-robin); separate from Phase 6 spot burst markets.
const POSITION_MARKETS: usize = 500;
const LIQ_MARKETS: usize = 100;
const BLOCKS_PER_SEC: u64 = 20;
const SETUP_BURST_RATE: u64 = 100_000;
const SPOT_ORDER_AMOUNT: u64 = 100_000;
const CHECKPOINT_TIMEOUT_SECS: u64 = 3600;
const CHECKPOINT_BURST_RATE: u64 = 1000;
const MEMPOOL_CHANNEL_CAP: usize = 16_384;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Burst Isolated margin liquidations + concurrent spot place_order load."
)]
struct Cli {
    #[clap(long, default_value = "127.0.0.1")]
    address: String,

    /// Target liquidations per block (clearinghouse budget / oracle window).
    #[clap(long, default_value = "200")]
    positions: usize,

    /// Total isolated positions to create.
    #[clap(long, default_value = "200000")]
    total_positions: usize,

    /// Number of spot markets to create (Phase 6 place_order burst).
    #[clap(long, default_value = "500")]
    num_markets: usize,

    /// Number of sender accounts to fund for parallel burst orders.
    #[clap(long, default_value = "1024")]
    senders: usize,

    /// Mempool send rate for the single shared connection (tx/s).
    #[clap(short, long, default_value = "500")]
    rate_per_task: u64,

    /// Duration to run burst trading / oracle window (seconds).
    #[clap(short, long, default_value = "10")]
    duration: u64,

    /// Still send ora_submit, but at healthy TRADE_PRICE (no crash → no liquidations).
    #[clap(long, default_value_t = false)]
    no_liquidate: bool,

    /// Skip staking committee setup (assume already promoted).
    #[clap(long, default_value_t = false)]
    skip_staking: bool,

    /// Append liquidated position events from NewBlock WS to this JSONL file.
    #[clap(long, default_value = "liquidations.jsonl")]
    liq_log: PathBuf,
}

#[derive(Clone)]
struct PositionFixture {
    #[allow(dead_code)]
    index: usize,
    market: ContractAddress,
    margin: ContractAddress,
}

#[derive(Debug, Clone)]
struct SpotMarketInfo {
    market_address: ContractAddress,
    base_token: ContractAddress,
    ask_price: u64,
}

struct BurstSpotSender {
    signer: Arc<Signer>,
    address: Address,
    market_index: usize,
}

fn quote_for_base(amount: u64, price: u64) -> u64 {
    ((amount as u128).saturating_mul(price as u128) / TOKEN_SCALE as u128) as u64
}

fn spot_fund_amount_per_sender(rate_per_task: u64, duration: u64) -> u64 {
    SPOT_ORDER_AMOUNT
        .saturating_mul(rate_per_task.max(1))
        .saturating_mul(duration.max(1))
        .saturating_add(SPOT_ORDER_AMOUNT)
}

/// Extra base funding so checkpoint place-order drive can run until tip catches up.
fn checkpoint_fund_amount_per_sender() -> u64 {
    let secs = CHECKPOINT_TIMEOUT_SECS.max(60).min(600);
    SPOT_ORDER_AMOUNT
        .saturating_mul(CHECKPOINT_BURST_RATE.max(1))
        .saturating_mul(secs)
        .saturating_add(SPOT_ORDER_AMOUNT)
}

fn market_ask_price(market_index: usize) -> u64 {
    let base = 10_000_000u64 + market_index as u64 * 1_000_000;
    base.saturating_add(100 * SPOT_TICK_SIZE)
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
    .map_err(|e| format!("create_token action: {e}"))?;
    let receipt = submit_ok(client, signer, sender, None, vec![action]).await?;
    extract_token_address_from_events(&receipt)
        .ok_or_else(|| "missing token_created event".to_string())
}

async fn transfer(
    client: &LightPoolClient,
    signer: &Signer,
    token: ContractAddress,
    to: Address,
    amount: u64,
) -> Result<(), String> {
    let action = ActionBuilder::transfer_token(token, TransferParams { to, amount })
        .map_err(|e| format!("transfer action: {e}"))?;
    submit_ok(client, signer, signer.address(), None, vec![action]).await?;
    Ok(())
}

fn spot_market_index(market: ContractAddress) -> u64 {
    let rest = market.rest();
    let mut bytes = [0u8; 8];
    bytes[1..8].copy_from_slice(&rest);
    u64::from_be_bytes(bytes)
}

fn margin_account_index(margin: ContractAddress) -> u64 {
    let rest = margin.rest();
    let mut bytes = [0u8; 8];
    bytes[2..8].copy_from_slice(&rest[1..7]);
    u64::from_be_bytes(bytes)
}

fn pool_index(pool: ContractAddress) -> u64 {
    margin_account_index(pool)
}

/// Load `~/.lightpool/wallet.json` so staking/oracle owner matches the node's validator registry.
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
    let signer = Signer::from_secret_key_bytes(&key)
        .map_err(|e| format!("Signer from default wallet: {e}"))?;
    if let Some(expected) = value.get("address").and_then(|v| v.as_str()) {
        let got = format!("{}", signer.address());
        if !got.eq_ignore_ascii_case(expected) {
            warn!(
                "default wallet address mismatch: file={expected} derived={got}"
            );
        }
    }
    Ok(signer)
}

/// Shared mempool ingress: producers enqueue tx bytes; one worker owns the TCP connection.
#[derive(Clone)]
struct MempoolOut {
    tx: mpsc::Sender<Bytes>,
}

impl MempoolOut {
    async fn send(&self, bytes: Bytes) -> Result<(), String> {
        self.tx
            .send(bytes)
            .await
            .map_err(|_| "mempool channel closed".to_string())
    }
}

fn spawn_mempool_worker(
    addr: String,
    rate: Arc<AtomicU64>,
) -> (MempoolOut, tokio::task::JoinHandle<()>) {
    let (tx, mut rx) = mpsc::channel::<Bytes>(MEMPOOL_CHANNEL_CAP);
    let handle = tokio::spawn(async move {
        let stream = match TcpStream::connect(&addr).await {
            Ok(s) => s,
            Err(e) => {
                error!("mempool connect {addr}: {e}");
                return;
            }
        };
        info!("Mempool worker connected to {addr} (single connection for all phases)");
        let mut transport = Framed::new(stream, LengthDelimitedCodec::new());
        let mut tokens = 0.0f64;
        let mut last = Instant::now();
        let mut last_rate = 0u64;
        while let Some(bytes) = rx.recv().await {
            // Reload rate every wait tick so Phase 5 (500/s) vs setup (100k/s) applies immediately.
            loop {
                let effective_u64 = rate.load(Ordering::Relaxed).max(1);
                let effective = effective_u64 as f64;
                let cap = effective.max(1.0);
                if effective_u64 != last_rate {
                    // Drop leftover burst capacity when rate drops (e.g. 100k → 500).
                    tokens = tokens.min(cap);
                    last_rate = effective_u64;
                }
                let now = Instant::now();
                tokens = (tokens + now.duration_since(last).as_secs_f64() * effective).min(cap);
                last = now;
                if tokens >= 1.0 {
                    tokens -= 1.0;
                    break;
                }
                let wait = Duration::from_secs_f64(((1.0 - tokens) / effective).max(0.0))
                    .max(Duration::from_micros(50))
                    .min(Duration::from_millis(5));
                tokio::time::sleep(wait).await;
            }
            if transport.send(bytes).await.is_err() {
                error!("mempool worker send failed; exiting");
                break;
            }
        }
        info!("Mempool worker stopped");
    });
    (MempoolOut { tx }, handle)
}

/// Enqueue `count` txs onto the shared mempool channel (rate-limited by the worker).
async fn build_and_spray<F>(
    out: &MempoolOut,
    label: &str,
    count: usize,
    expiration: &mut u64,
    mut build_one: F,
) -> Result<u64, String>
where
    F: FnMut(usize, u64) -> Result<Vec<u8>, String>,
{
    if count == 0 {
        return Ok(0);
    }
    let mut sent = 0u64;
    for idx in 0..count {
        let bytes = build_one(idx, *expiration)?;
        *expiration = expiration.saturating_sub(1);
        out.send(Bytes::from(bytes))
            .await
            .map_err(|e| format!("{label}: {e}"))?;
        sent += 1;
        if sent == count as u64 || sent % 16_384 == 0 {
            info!("{label}: queued {sent}/{count}");
        }
    }
    Ok(sent)
}

fn sign_actions_tx(
    signer: &Signer,
    sender: Address,
    account: Option<Address>,
    actions: Vec<lightpool_sdk::Action>,
    expiration: u64,
) -> Result<Vec<u8>, String> {
    let mut tx = TransactionBuilder::new()
        .sender(sender)
        .expiration(expiration);
    if let Some(account) = account {
        tx = tx.account(account);
    }
    for action in actions {
        tx = tx.add_action(action);
    }
    let signed = tx
        .build_and_sign_only(signer)
        .map_err(|e| format!("sign setup tx: {e}"))?;
    bincode::serialize(&signed).map_err(|e| format!("serialize setup tx: {e}"))
}

fn unsigned_actions_tx(
    sender: Address,
    account: Option<Address>,
    actions: Vec<lightpool_sdk::Action>,
    expiration: u64,
) -> Result<Vec<u8>, String> {
    let mut tx = TransactionBuilder::new()
        .sender(sender)
        .expiration(expiration);
    if let Some(account) = account {
        tx = tx.account(account);
    }
    for action in actions {
        tx = tx.add_action(action);
    }
    let built = tx
        .build_and_without_sign()
        .map_err(|e| format!("build unsigned setup tx: {e}"))?;
    bincode::serialize(&built).map_err(|e| format!("serialize setup tx: {e}"))
}

/// Fast path: mempool-burst create markets / fund / margins / trades, predict addresses by index.
async fn setup_positions_mempool_burst(
    client: &LightPoolClient,
    out: &MempoolOut,
    lender: &Arc<Signer>,
    seller: &Arc<Signer>,
    bidder: &Arc<Signer>,
    usdt: ContractAddress,
    btc: ContractAddress,
    pools: &[ContractAddress],
    total_positions: usize,
    num_borrowers: usize,
    num_markets: usize,
) -> Result<Vec<PositionFixture>, String> {
    if pools.is_empty() {
        return Err("pools must be non-empty".into());
    }
    let num_borrowers = num_borrowers.max(1).min(total_positions);
    let num_markets = num_markets.max(1).min(total_positions);
    let per_borrower_positions =
        (total_positions + num_borrowers - 1) / num_borrowers;
    let fund_each = COLLATERAL.saturating_mul(per_borrower_positions as u64);
    let setup_start = Instant::now();
    let keygen_start = Instant::now();
    let borrowers: Vec<Arc<Signer>> = (0..num_borrowers)
        .map(|_| Arc::new(Signer::new()))
        .collect();
    info!(
        "Generated {num_borrowers} borrower keys in {:?}",
        keygen_start.elapsed()
    );
    let borrower_for = |i: usize| -> &Arc<Signer> { &borrowers[i % num_borrowers] };

    // Probe first market index via RPC.
    let probe_market_action = ActionBuilder::create_market(CreateMarketParams {
        name: "LiqMktProbe".into(),
        base_token: btc,
        quote_token: usdt,
        min_order_size: MIN_ORDER_SIZE,
        tick_size: TICK_SIZE,
        maker_fee_bps: 0,
        taker_fee_bps: 0,
        allow_market_orders: true,
        state: MarketState::Active,
        limit_order: true,
        side_book_size: SegmentSize::Large,
        creator: lender.address(),
    })
    .map_err(|e| e.to_string())?;
    let probe_receipt = submit_ok(
        client,
        lender.as_ref(),
        lender.address(),
        None,
        vec![probe_market_action],
    )
    .await?;
    let probe_market = extract_market_address_from_events(&probe_receipt)
        .ok_or_else(|| "missing probe market".to_string())?;
    let market_start = spot_market_index(probe_market);
    info!("Probe market index start={market_start} (creating {num_markets} markets total)");

    // Burst remaining create_market for shared market pool only.
    let mut expiration = u64::MAX;
    let market_sent = build_and_spray(
        out,
        "create_market",
        num_markets.saturating_sub(1),
        &mut expiration,
        |j, exp| {
            let action = ActionBuilder::create_market(CreateMarketParams {
                name: format!("LiqMkt{}", j + 2).into(),
                base_token: btc,
                quote_token: usdt,
                min_order_size: MIN_ORDER_SIZE,
                tick_size: TICK_SIZE,
                maker_fee_bps: 0,
                taker_fee_bps: 0,
                allow_market_orders: true,
                state: MarketState::Active,
                limit_order: true,
                side_book_size: SegmentSize::Large,
                creator: lender.address(),
            })
            .map_err(|e| e.to_string())?;
            sign_actions_tx(
                lender.as_ref(),
                lender.address(),
                None,
                vec![action],
                exp,
            )
        },
    )
    .await?;
    info!("create_market burst sent {market_sent}");

    let markets: Vec<ContractAddress> = (0..num_markets)
        .map(|i| {
            market_contract(market_start + i as u64)
                .unwrap_or_else(|_| unreachable!("market index in range"))
        })
        .collect();
    let market_for = |i: usize| markets[i % num_markets];

    // Fund borrowers once each (enough collateral for their share of positions).
    let fund_sent = build_and_spray(
        out,
        "fund",
        num_borrowers,
        &mut expiration,
        |j, exp| {
            let action = ActionBuilder::transfer_token(
                usdt,
                TransferParams {
                    to: borrowers[j].address(),
                    amount: fund_each,
                },
            )
            .map_err(|e| e.to_string())?;
            sign_actions_tx(
                lender.as_ref(),
                lender.address(),
                None,
                vec![action],
                exp,
            )
        },
    )
    .await?;
    info!("fund borrowers burst sent {fund_sent} ({} each)", fund_each);

    // Probe first margin index (markets[0] already exists from RPC probe).
    let create_margin0 = ActionBuilder::create_margin_account(CreateMarginParams {
        pool: pools[0],
        mode: MARGIN_MODE_ISOLATED,
        market: Some(market_for(0)),
    })
    .map_err(|e| e.to_string())?;
    let margin0_receipt = submit_ok(
        client,
        borrower_for(0).as_ref(),
        borrower_for(0).address(),
        None,
        vec![create_margin0],
    )
    .await?;
    let margin0 = extract_margin_created_from_events(&margin0_receipt)
        .ok_or_else(|| "missing probe margin".to_string())?
        .margin;
    let margin_start = margin_account_index(margin0);
    info!("Probe margin index start={margin_start}");

    let margin_sent = build_and_spray(
        out,
        "create_margin",
        total_positions.saturating_sub(1),
        &mut expiration,
        |j, exp| {
            let i = j + 1;
            let borrower = borrower_for(i);
            let action = ActionBuilder::create_margin_account(CreateMarginParams {
                pool: pools[i % pools.len()],
                mode: MARGIN_MODE_ISOLATED,
                market: Some(market_for(i)),
            })
            .map_err(|e| e.to_string())?;
            unsigned_actions_tx(borrower.address(), None, vec![action], exp)
        },
    )
    .await?;
    info!("create_margin burst sent {margin_sent}");

    let margins: Vec<ContractAddress> = (0..total_positions)
        .map(|i| {
            margin_account_contract(margin_start + i as u64)
                .unwrap_or_else(|_| unreachable!("margin index in range"))
        })
        .collect();

    let db_sent = build_and_spray(
        out,
        "deposit+borrow",
        total_positions,
        &mut expiration,
        |i, exp| {
            let borrower = borrower_for(i);
            let deposit = ActionBuilder::deposit_margin_collateral(
                margins[i],
                DepositCollateralParams {
                    amount: COLLATERAL,
                },
            )
            .map_err(|e| e.to_string())?;
            let borrow = ActionBuilder::borrow_margin(
                margins[i],
                BorrowParams {
                    amount: BORROW_AMOUNT,
                },
            )
            .map_err(|e| e.to_string())?;
            unsigned_actions_tx(
                borrower.address(),
                None,
                vec![deposit, borrow],
                exp,
            )
        },
    )
    .await?;
    info!("deposit+borrow burst sent {db_sent}");

    // seller lists, margin buys, bidder adds liq (3 txs per position; chunk by position count * 3 via flat index)
    let trade_sent = build_and_spray(
        out,
        "open+bid",
        total_positions * 3,
        &mut expiration,
        |flat, exp| {
            let i = flat / 3;
            let kind = flat % 3;
            let market = market_for(i);
            match kind {
                0 => {
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
                    unsigned_actions_tx(seller.address(), None, vec![sell], exp)
                }
                1 => {
                    let borrower = borrower_for(i);
                    let trading = margin_trading_account(margins[i]);
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
                    unsigned_actions_tx(
                        borrower.address(),
                        Some(trading),
                        vec![buy],
                        exp,
                    )
                }
                _ => {
                    let bid = ActionBuilder::place_order(
                        market,
                        PlaceOrderParams {
                            side: OrderSide::Buy,
                            amount: TRADE_AMOUNT,
                            order_type: OrderParamsType::Limit {
                                tif: TimeInForce::GTC,
                            },
                            limit_price: TRADE_PRICE,
                            token_address: usdt,
                        },
                    )
                    .map_err(|e| e.to_string())?;
                    unsigned_actions_tx(bidder.address(), None, vec![bid], exp)
                }
            }
        },
    )
    .await?;
    info!("open+bid burst sent {trade_sent}");

    let fixtures: Vec<PositionFixture> = (0..total_positions)
        .map(|i| PositionFixture {
            index: i,
            market: market_for(i),
            margin: margins[i],
        })
        .collect();
    info!(
        "Mempool position setup done: {} fixtures on {} markets in {:?}",
        fixtures.len(),
        num_markets,
        setup_start.elapsed()
    );
    Ok(fixtures)
}


/// Spray ora_submit high→low via shared mempool channel.
/// Use `CRASH_PRICE` to drive liquidations, or a healthy mark (e.g. `TRADE_PRICE`).
async fn mempool_oracle_price_burst(
    out: MempoolOut,
    lender: Arc<Signer>,
    markets: Arc<Vec<ContractAddress>>,
    per_block: usize,
    blocks_per_sec: u64,
    duration_secs: u64,
    oracle_price: u64,
    counter: Arc<AtomicU64>,
) -> Result<(), String> {
    if markets.is_empty() {
        return Ok(());
    }
    let per_block = per_block.max(1);
    let blocks_per_sec = blocks_per_sec.max(1);
    let rate_per_second = (per_block as u64).saturating_mul(blocks_per_sec).max(1);
    info!(
        "Oracle mempool burst: {} markets high→low @ price={}, ~{}/block × {} blocks/s → pace≈{}/s, duration={}s",
        markets.len(),
        oracle_price,
        per_block,
        blocks_per_sec,
        rate_per_second,
        duration_secs
    );
    let end_time = Instant::now() + Duration::from_secs(duration_secs.max(1));
    let interval = Duration::from_secs_f64(1.0 / rate_per_second as f64);
    let mut next_rev: usize = 0;
    let mut tx_count = 0u64;
    let mut expiration = u64::MAX;

    while Instant::now() < end_time && next_rev < markets.len() {
        let idx = markets.len() - 1 - next_rev;
        next_rev += 1;
        let market = markets[idx];
        let action = ActionBuilder::submit_oracle_price(
            market,
            SubmitOraclePriceParams {
                price: oracle_price,
            },
        )
        .map_err(|e| format!("oracle action: {e}"))?;
        let tx = TransactionBuilder::new()
            .sender(lender.address())
            .expiration(expiration)
            .add_action(action)
            .build_and_sign_only(lender.as_ref())
            .map_err(|e| format!("oracle sign: {e}"))?;
        let tx_bytes = bincode::serialize(&tx).map_err(|e| format!("oracle serialize: {e}"))?;
        if let Err(e) = out.send(Bytes::from(tx_bytes)).await {
            warn!("oracle mempool enqueue failed: {e}");
            break;
        }
        tx_count += 1;
        expiration = expiration.saturating_sub(1);
        counter.fetch_add(1, Ordering::Relaxed);
        tokio::time::sleep(interval).await;
    }
    info!(
        "Oracle mempool burst queued {tx_count} ora_submit txs (covered {}/{})",
        next_rev.min(markets.len()),
        markets.len()
    );
    Ok(())
}

async fn rpc_get_committed_block_num(rpc: &str) -> Result<u64, String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "getSyncInfo",
        "params": [],
        "id": 1
    });
    let client = reqwest::Client::new();
    let response = client
        .post(rpc)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("getSyncInfo http: {e}"))?;
    let value: serde_json::Value = response
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

async fn wait_for_checkpoint(rpc: &str, target: u64, timeout_secs: u64) -> Result<u64, String> {
    info!("Waiting for committed_block_num >= {target} (first/next checkpoint)...");
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut last = 0u64;
    while Instant::now() < deadline {
        match rpc_get_committed_block_num(rpc).await {
            Ok(tip) => {
                last = tip;
                if tip >= target {
                    info!("Checkpoint reached: committed_block_num={tip}");
                    return Ok(tip);
                }
                info!("Waiting checkpoint: tip={tip} target={target}");
            }
            Err(e) => warn!("getSyncInfo failed: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(format!(
        "timed out waiting for checkpoint >= {target} (last tip={last})"
    ))
}


async fn place_order_until_stop(
    out: MempoolOut,
    senders: Arc<Vec<BurstSpotSender>>,
    markets: Arc<Vec<SpotMarketInfo>>,
    order_amount: u64,
    stop: Arc<AtomicBool>,
    counter: Arc<AtomicU64>,
) -> Result<(), String> {
    if senders.is_empty() || markets.is_empty() {
        return Ok(());
    }
    info!("Checkpoint place_order → shared mempool channel, senders={}", senders.len());
    let mut tx_count = 0u64;
    let mut expiration = u64::MAX;
    while !stop.load(Ordering::Relaxed) {
        let sender = &senders[tx_count as usize % senders.len()];
        let market = &markets[sender.market_index];
        let action = ActionBuilder::place_order(
            market.market_address,
            PlaceOrderParams {
                side: OrderSide::Sell,
                amount: order_amount,
                order_type: OrderParamsType::Limit {
                    tif: TimeInForce::GTC,
                },
                limit_price: market.ask_price,
                token_address: market.base_token,
            },
        )
        .map_err(|e| format!("checkpoint place_order action: {e}"))?;
        let tx = TransactionBuilder::new()
            .sender(sender.address)
            .expiration(expiration)
            .add_action(action)
            .build_and_without_sign()
            .map_err(|e| format!("checkpoint build tx: {e}"))?;
        let tx_bytes =
            bincode::serialize(&tx).map_err(|e| format!("checkpoint serialize: {e}"))?;
        if let Err(e) = out.send(Bytes::from(tx_bytes)).await {
            warn!("checkpoint mempool enqueue failed: {e}");
            break;
        }
        tx_count += 1;
        expiration = expiration.saturating_sub(1);
        counter.fetch_add(1, Ordering::Relaxed);
    }
    info!("Checkpoint place_order stopped after {tx_count}");
    Ok(())
}

/// Place-order burst until tip reaches checkpoint target (single shared mempool channel).
async fn drive_past_checkpoint_with_place_orders(
    rpc: &str,
    out: MempoolOut,
    send_rate: Arc<AtomicU64>,
    senders: Arc<Vec<BurstSpotSender>>,
    markets: Arc<Vec<SpotMarketInfo>>,
    target: u64,
    timeout_secs: u64,
    order_amount: u64,
) -> Result<u64, String> {
    let tip_now = rpc_get_committed_block_num(rpc).await.unwrap_or(0);
    if tip_now >= target {
        info!("Tip already past checkpoint ({tip_now} >= {target}); skip checkpoint place-order");
        return Ok(tip_now);
    }
    if senders.is_empty() || markets.is_empty() {
        return wait_for_checkpoint(rpc, target, timeout_secs).await;
    }

    send_rate.store(CHECKPOINT_BURST_RATE.max(1), Ordering::Relaxed);
    info!(
        "Driving tip to checkpoint with place_order: tip={tip_now} target={target}          rate={}/s senders={}",
        CHECKPOINT_BURST_RATE,
        senders.len()
    );

    let stop = Arc::new(AtomicBool::new(false));
    let counter = Arc::new(AtomicU64::new(0));
    let handle = tokio::spawn(place_order_until_stop(
        out,
        Arc::clone(&senders),
        Arc::clone(&markets),
        order_amount,
        Arc::clone(&stop),
        Arc::clone(&counter),
    ));

    let tip = match wait_for_checkpoint(rpc, target, timeout_secs).await {
        Ok(tip) => tip,
        Err(e) => {
            stop.store(true, Ordering::Relaxed);
            let _ = handle.await;
            return Err(e);
        }
    };
    stop.store(true, Ordering::Relaxed);
    let _ = handle.await;
    info!(
        "Checkpoint place-order drive done: tip={tip} sent≈{}",
        counter.load(Ordering::Relaxed)
    );
    Ok(tip)
}

async fn measure_baseline_latency(
    client: &LightPoolClient,
    creator: &Signer,
) -> Result<Duration, String> {
    info!("Measuring baseline processing latency with token creation...");
    let start = Instant::now();
    let _ = create_token(client, creator, "BaselineLatency", "BASE", 1).await?;
    let latency = start.elapsed();
    info!("Baseline latency: {:.3}s", latency.as_secs_f64());
    Ok(latency)
}

fn build_burst_senders(senders: usize, num_markets: usize) -> Vec<BurstSpotSender> {
    (0..senders)
        .map(|index| {
            let signer = Arc::new(Signer::new());
            let address = signer.address();
            BurstSpotSender {
                signer,
                address,
                market_index: index % num_markets,
            }
        })
        .collect()
}

/// Mempool-burst margin pools: RPC probe pool0+supply, spray the rest, predict addresses.
async fn setup_pools_mempool_burst(
    client: &LightPoolClient,
    out: &MempoolOut,
    lender: &Signer,
    usdt: ContractAddress,
    num_pools: usize,
    per_pool_supply: u64,
) -> Result<Vec<ContractAddress>, String> {
    let num_pools = num_pools.max(1);
    info!(
        "Creating {num_pools} funding pools + supply ({per_pool_supply} each) via mempool..."
    );

    let create_pool0 = ActionBuilder::create_margin_pool(CreatePoolParams {
        token: usdt,
        max_ltv_bps: 8_000,
        maint_bps: 8_500,
        liq_bonus_bps: 500,
    })
    .map_err(|e| e.to_string())?;
    let pool0_receipt = submit_ok(
        client,
        lender,
        lender.address(),
        None,
        vec![create_pool0],
    )
    .await
    .map_err(|e| format!("create_pool[0]: {e}"))?;
    let pool0 = extract_pool_created_from_events(&pool0_receipt)
        .ok_or_else(|| "missing margin_pool_created for pool 0".to_string())?
        .pool;
    let pool_start = pool_index(pool0);
    info!("Probe pool index start={pool_start}");

    let supply0 = ActionBuilder::supply_margin_pool(
        pool0,
        SupplyParams {
            amount: per_pool_supply,
        },
    )
    .map_err(|e| e.to_string())?;
    submit_ok(client, lender, lender.address(), None, vec![supply0])
        .await
        .map_err(|e| format!("supply_pool[0]: {e}"))?;

    let remaining = num_pools.saturating_sub(1);
    let mut expiration = u64::MAX;
    let setup_sent = build_and_spray(
        out,
        "pool_setup",
        remaining * 2,
        &mut expiration,
        |flat, exp| {
            let i = 1 + flat / 2;
            let kind = flat % 2;
            match kind {
                0 => {
                    let action = ActionBuilder::create_margin_pool(CreatePoolParams {
                        token: usdt,
                        max_ltv_bps: 8_000,
                        maint_bps: 8_500,
                        liq_bonus_bps: 500,
                    })
                    .map_err(|e| e.to_string())?;
                    sign_actions_tx(lender, lender.address(), None, vec![action], exp)
                }
                _ => {
                    let pool = pool_contract(pool_start + i as u64)
                        .unwrap_or_else(|_| unreachable!("pool index in range"));
                    let action = ActionBuilder::supply_margin_pool(
                        pool,
                        SupplyParams {
                            amount: per_pool_supply,
                        },
                    )
                    .map_err(|e| e.to_string())?;
                    sign_actions_tx(lender, lender.address(), None, vec![action], exp)
                }
            }
        },
    )
    .await?;
    info!("pool_setup burst sent {setup_sent}");

    Ok((0..num_pools)
        .map(|i| {
            pool_contract(pool_start + i as u64)
                .unwrap_or_else(|_| unreachable!("pool index in range"))
        })
        .collect())
}

/// Mempool-burst spot markets: RPC probe token0/quote0/market0, spray the rest, predict addresses.
async fn setup_spot_burst_markets(
    client: &LightPoolClient,
    out: &MempoolOut,
    creator: &Signer,
    num_markets: usize,
    fund_amount: u64,
    senders_n: usize,
) -> Result<Vec<SpotMarketInfo>, String> {
    let num_markets = num_markets.max(1);
    info!(
        "Setting up {num_markets} spot markets via shared mempool channel..."
    );
    let senders_on_market =
        |i: usize| senders_n / num_markets + usize::from(i < senders_n % num_markets);
    let base_supply = |i: usize| {
        fund_amount
            .saturating_mul(senders_on_market(i) as u64)
            .saturating_add(fund_amount)
    };

    let probe_base = create_token(
        client,
        creator,
        "SpotBase0",
        "SB0",
        base_supply(0),
    )
    .await?;
    let token_start = spot_market_index(probe_base);
    let probe_quote = create_token(client, creator, "SpotQuote0", "SQ0", 1).await?;
    let quote0_idx = spot_market_index(probe_quote);
    if quote0_idx != token_start + 1 {
        return Err(format!(
            "spot quote0 index {quote0_idx} != token_start+1 {}",
            token_start + 1
        ));
    }
    info!("Probe token index start={token_start}");

    let probe_market_action = ActionBuilder::create_market(CreateMarketParams {
        name: "SpotBurst0".into(),
        base_token: probe_base,
        quote_token: probe_quote,
        min_order_size: MIN_ORDER_SIZE,
        tick_size: SPOT_TICK_SIZE,
        maker_fee_bps: 10,
        taker_fee_bps: 20,
        allow_market_orders: true,
        state: MarketState::Active,
        limit_order: true,
        side_book_size: SegmentSize::Large,
        creator: creator.address(),
    })
    .map_err(|e| e.to_string())?;
    let probe_receipt = submit_ok(
        client,
        creator,
        creator.address(),
        None,
        vec![probe_market_action],
    )
    .await?;
    let probe_market = extract_market_address_from_events(&probe_receipt)
        .ok_or_else(|| "missing spot burst probe market".to_string())?;
    let market_start = spot_market_index(probe_market);
    info!("Probe spot market index start={market_start}");

    let mut expiration = u64::MAX;
    let remaining_markets = num_markets.saturating_sub(1);
    // One connection: per market base → quote → create_market (FIFO within each triple).
    let setup_sent = build_and_spray(
        out,
        "spot_market_setup",
        remaining_markets * 3,
        &mut expiration,
        |flat, exp| {
            let i = 1 + flat / 3;
            let kind = flat % 3;
            match kind {
                0 => {
                    let action = ActionBuilder::create_token(CreateTokenParams {
                        name: format!("SpotBase{i}").into(),
                        symbol: format!("SB{i}").into(),
                        total_supply: base_supply(i),
                        mintable: true,
                        to: creator.address(),
                    })
                    .map_err(|e| e.to_string())?;
                    sign_actions_tx(creator, creator.address(), None, vec![action], exp)
                }
                1 => {
                    let action = ActionBuilder::create_token(CreateTokenParams {
                        name: format!("SpotQuote{i}").into(),
                        symbol: format!("SQ{i}").into(),
                        total_supply: 1,
                        mintable: true,
                        to: creator.address(),
                    })
                    .map_err(|e| e.to_string())?;
                    sign_actions_tx(creator, creator.address(), None, vec![action], exp)
                }
                _ => {
                    let base = token_contract(token_start + (i as u64) * 2)
                        .unwrap_or_else(|_| unreachable!("token index in range"));
                    let quote = token_contract(token_start + (i as u64) * 2 + 1)
                        .unwrap_or_else(|_| unreachable!("token index in range"));
                    let action = ActionBuilder::create_market(CreateMarketParams {
                        name: format!("SpotBurst{i}").into(),
                        base_token: base,
                        quote_token: quote,
                        min_order_size: MIN_ORDER_SIZE,
                        tick_size: SPOT_TICK_SIZE,
                        maker_fee_bps: 10,
                        taker_fee_bps: 20,
                        allow_market_orders: true,
                        state: MarketState::Active,
                        limit_order: true,
                        side_book_size: SegmentSize::Large,
                        creator: creator.address(),
                    })
                    .map_err(|e| e.to_string())?;
                    sign_actions_tx(creator, creator.address(), None, vec![action], exp)
                }
            }
        },
    )
    .await?;
    info!("spot_market_setup burst sent {setup_sent}");

    Ok((0..num_markets)
        .map(|i| SpotMarketInfo {
            market_address: market_contract(market_start + i as u64)
                .unwrap_or_else(|_| unreachable!("market index in range")),
            base_token: token_contract(token_start + (i as u64) * 2)
                .unwrap_or_else(|_| unreachable!("token index in range")),
            ask_price: market_ask_price(i),
        })
        .collect())
}

async fn fund_burst_senders(
    out: &MempoolOut,
    creator: &Signer,
    senders: &[BurstSpotSender],
    markets: &[SpotMarketInfo],
    fund_amount: u64,
) -> Result<(), String> {
    if senders.is_empty() {
        return Ok(());
    }
    info!(
        "Funding {} spot-burst senders with {} base each via mempool...",
        senders.len(),
        fund_amount
    );
    let mut expiration = u64::MAX;
    let fund_sent = build_and_spray(
        out,
        "spot_fund",
        senders.len(),
        &mut expiration,
        |j, exp| {
            let sender = &senders[j];
            let market = &markets[sender.market_index];
            let action = ActionBuilder::transfer_token(
                market.base_token,
                TransferParams {
                    to: sender.address,
                    amount: fund_amount,
                },
            )
            .map_err(|e| format!("fund transfer action: {e}"))?;
            sign_actions_tx(creator, creator.address(), None, vec![action], exp)
        },
    )
    .await?;
    info!("spot_fund burst sent {fund_sent}");
    Ok(())
}

fn measure_place_order_tx_size(
    sender: &BurstSpotSender,
    market: &SpotMarketInfo,
    order_amount: u64,
) -> Result<usize, String> {
    let action = ActionBuilder::place_order(
        market.market_address,
        PlaceOrderParams {
            side: OrderSide::Sell,
            amount: order_amount,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::GTC,
            },
            limit_price: market.ask_price,
            token_address: market.base_token,
        },
    )
    .map_err(|e| e.to_string())?;
    let tx = TransactionBuilder::new()
        .sender(sender.address)
        .expiration(u64::MAX)
        .add_action(action)
        .build_and_without_sign()
        .map_err(|e| e.to_string())?;
    let bytes = bincode::serialize(&tx).map_err(|e| e.to_string())?;
    Ok(bytes.len())
}


async fn spot_place_order_burst(
    out: MempoolOut,
    senders: Arc<Vec<BurstSpotSender>>,
    markets: Arc<Vec<SpotMarketInfo>>,
    duration_secs: u64,
    order_amount: u64,
    counter: Arc<AtomicU64>,
) -> Result<(), String> {
    if senders.is_empty() || markets.is_empty() {
        return Ok(());
    }
    info!(
        "Spot burst → shared mempool channel, senders={} duration={}s",
        senders.len(),
        duration_secs
    );
    let end_time = Instant::now() + Duration::from_secs(duration_secs);
    let mut tx_count = 0u64;
    let mut expiration = u64::MAX;
    while Instant::now() < end_time {
        let sender = &senders[tx_count as usize % senders.len()];
        let market = &markets[sender.market_index];
        let action = ActionBuilder::place_order(
            market.market_address,
            PlaceOrderParams {
                side: OrderSide::Sell,
                amount: order_amount,
                order_type: OrderParamsType::Limit {
                    tif: TimeInForce::GTC,
                },
                limit_price: market.ask_price,
                token_address: market.base_token,
            },
        )
        .map_err(|e| format!("spot place_order action: {e}"))?;
        let tx = TransactionBuilder::new()
            .sender(sender.address)
            .expiration(expiration)
            .add_action(action)
            .build_and_without_sign()
            .map_err(|e| format!("spot build tx: {e}"))?;
        let tx_bytes = bincode::serialize(&tx).map_err(|e| format!("spot serialize: {e}"))?;
        if let Err(e) = out.send(Bytes::from(tx_bytes)).await {
            warn!("spot mempool enqueue failed: {e}");
            break;
        }
        tx_count += 1;
        expiration = expiration.saturating_sub(1);
        counter.fetch_add(1, Ordering::Relaxed);
    }
    info!("Spot burst queued {tx_count} place_orders");
    Ok(())
}

fn spawn_spot_place_order_burst(
    out: MempoolOut,
    senders: Arc<Vec<BurstSpotSender>>,
    markets: Arc<Vec<SpotMarketInfo>>,
    duration: u64,
    order_amount: u64,
) -> (
    tokio::task::JoinHandle<Result<(), String>>,
    tokio::task::JoinHandle<()>,
    Arc<AtomicU64>,
    Instant,
) {
    let counter = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();
    let handle = tokio::spawn(spot_place_order_burst(
        out,
        Arc::clone(&senders),
        Arc::clone(&markets),
        duration,
        order_amount,
        Arc::clone(&counter),
    ));
    let monitor_counter = Arc::clone(&counter);
    let monitor = tokio::spawn(async move {
        let mut last_count = 0u64;
        let mut last_time = Instant::now();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        for i in 0..duration {
            interval.tick().await;
            let current = monitor_counter.load(Ordering::Relaxed);
            let elapsed = last_time.elapsed().as_secs_f64().max(1e-9);
            let rate = (current - last_count) as f64 / elapsed;
            info!(
                "Spot burst progress [{:2}/{}]: {} orders, {:.1} orders/s",
                i + 1,
                duration,
                current,
                rate
            );
            last_count = current;
            last_time = Instant::now();
        }
    });
    (handle, monitor, counter, start_time)
}

async fn finalize_spot_burst_metrics(
    client: &LightPoolClient,
    senders: &Arc<Vec<BurstSpotSender>>,
    markets: &Arc<Vec<SpotMarketInfo>>,
    order_amount: u64,
    handle: tokio::task::JoinHandle<Result<(), String>>,
    monitor: tokio::task::JoinHandle<()>,
    counter: Arc<AtomicU64>,
    start_time: Instant,
    baseline_latency: Duration,
) {
    let mempool_send_time = start_time.elapsed();
    match handle.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => error!("Spot burst failed: {e}"),
        Err(e) => error!("Spot burst panicked: {e}"),
    }
    monitor.abort();
    let total_orders = counter.load(Ordering::Relaxed);
    let mempool_send_rate = total_orders as f64 / mempool_send_time.as_secs_f64().max(1e-9);
    info!("Spot mempool phase:");
    info!("   Total orders sent: {total_orders}");
    info!(
        "   Mempool send time: {:.2}s",
        mempool_send_time.as_secs_f64()
    );
    info!("   Mempool send rate: {mempool_send_rate:.1} orders/s");

    info!("Sending final RPC place_order to measure completion latency...");
    let final_ok = if let (Some(sender), Some(market)) = (
        senders.first(),
        senders
            .first()
            .and_then(|s| markets.get(s.market_index)),
    ) {
        let action = ActionBuilder::place_order(
            market.market_address,
            PlaceOrderParams {
                side: OrderSide::Sell,
                amount: order_amount,
                order_type: OrderParamsType::Limit {
                    tif: TimeInForce::GTC,
                },
                limit_price: market.ask_price,
                token_address: market.base_token,
            },
        );
        match action {
            Ok(action) => {
                match submit_ok(
                    client,
                    sender.signer.as_ref(),
                    sender.address,
                    None,
                    vec![action],
                )
                .await
                {
                    Ok(_) => {
                        info!("Final RPC place_order SUCCESS");
                        true
                    }
                    Err(e) => {
                        warn!("Final RPC place_order failed: {e}");
                        false
                    }
                }
            }
            Err(e) => {
                warn!("Final place_order action failed: {e}");
                false
            }
        }
    } else {
        false
    };

    let actual_completion_time = start_time.elapsed();
    let total_orders = counter.load(Ordering::Relaxed);
    let actual_tps = total_orders as f64 / actual_completion_time.as_secs_f64().max(1e-9);
    info!("========================================");
    info!("Concurrent spot place_order metrics");
    info!("========================================");
    info!(
        "Final RPC status: {}",
        if final_ok { "SUCCESS" } else { "FAILED" }
    );
    info!("Total orders sent: {total_orders}");
    info!(
        "Actual completion time: {:.2}s",
        actual_completion_time.as_secs_f64()
    );
    info!("Actual TPS: {actual_tps:.1} tx/s");
    info!(
        "Baseline latency: {:.3}s",
        baseline_latency.as_secs_f64()
    );
}

/// Subscribe to NewBlocks; append each ClearingHouseEvent::Liquidated as one JSONL line.
async fn run_liquidation_ws_logger(
    ws_url: String,
    out_path: PathBuf,
    stop: Arc<AtomicBool>,
    counter: Arc<AtomicU64>,
) -> Result<(), String> {
    let mut ws_client = WebSocketClient::new(Some(ws_url.clone()))
        .await
        .map_err(|e| format!("ws client: {e}"))?;
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let sub_id = ws_client
        .subscribe(Subscription::NewBlocks, sender)
        .await
        .map_err(|e| format!("subscribe NewBlocks: {e}"))?;
    info!(
        "Liquidation WS logger subscribed ({sub_id}) → {}",
        out_path.display()
    );

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&out_path)
        .map_err(|e| format!("open {}: {e}", out_path.display()))?;

    let mut blocks_seen = 0u64;
    let mut blocks_with_liq = 0u64;
    let mut parse_ok = 0u64;

    let mut write_block = |block: &lightpool_sdk::ReceiptBlock| -> Result<(), String> {
        blocks_seen += 1;
        parse_ok += 1;
        if block.clearinghouse_events.is_empty() {
            return Ok(());
        }
        blocks_with_liq += 1;
        for ev in &block.clearinghouse_events {
            let ClearingHouseEvent::Liquidated {
                margin,
                liquidator,
                repay_amount,
                seized_amount,
                debt,
            } = ev;
            let line = json!({
                "block_num": block.block_num,
                "margin": margin.to_string(),
                "liquidator": liquidator.to_string(),
                "repay_amount": repay_amount,
                "seized_amount": seized_amount,
                "debt": debt,
            });
            writeln!(file, "{line}")
                .map_err(|e| format!("write {}: {e}", out_path.display()))?;
            counter.fetch_add(1, Ordering::Relaxed);
        }
        let _ = file.flush();
        Ok(())
    };

    // Recv + write until main signals stop (or WS closes / errors).
    while !stop.load(Ordering::Relaxed) {
        tokio::select! {
            msg = receiver.recv() => {
                match msg {
                    Some(Message::NewBlock(block) | Message::ReceiptBlock(block)) => {
                        write_block(&block)?;
                    }
                    Some(Message::Error(err)) => {
                        warn!("Liquidation WS error: {err}");
                        break;
                    }
                    None => break,
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    }

    // After stop: drain late NewBlocks for a few seconds.
    info!("Liquidation WS logger draining for 5s after stop...");
    let drain_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < drain_deadline {
        match tokio::time::timeout(Duration::from_millis(200), receiver.recv()).await {
            Ok(Some(Message::NewBlock(block) | Message::ReceiptBlock(block))) => {
                write_block(&block)?;
            }
            Ok(Some(Message::Error(err))) => {
                warn!("Liquidation WS error during drain: {err}");
                break;
            }
            Ok(None) => break,
            Err(_) => {}
        }
    }

    let _ = file.flush();
    info!(
        "Liquidation WS logger stopped; blocks={} parsed={} with_liq={} events={} → {}",
        blocks_seen,
        parse_ok,
        blocks_with_liq,
        counter.load(Ordering::Relaxed),
        out_path.display()
    );
    Ok(())
}

/// Wait until liquidations appear or tip advances enough after ora_submit.
async fn wait_for_liquidation_window(
    rpc: &str,
    tip_before: u64,
    liq_count: Arc<AtomicU64>,
    min_tip_delta: u64,
    timeout_secs: u64,
) {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
    let target_tip = tip_before.saturating_add(min_tip_delta);
    loop {
        let events = liq_count.load(Ordering::Relaxed);
        let tip = rpc_get_committed_block_num(rpc).await.unwrap_or(tip_before);
        if events > 0 {
            info!(
                "Liquidation window: saw {events} events (tip {tip_before}→{tip})"
            );
            return;
        }
        if tip >= target_tip {
            info!(
                "Liquidation window: tip advanced {tip_before}→{tip}, events still 0"
            );
            return;
        }
        if Instant::now() >= deadline {
            warn!(
                "Liquidation window timed out after {timeout_secs}s (tip {tip_before}→{tip}, events=0)"
            );
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
async fn setup_staking_committee(
    client: &LightPoolClient,
    validator: &Signer,
) -> Result<ContractAddress, String> {
    info!("Staking: create LPL + init_config + register + bond + allocate Committee...");
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
        Err(e) => {
            // Already initialized on this chain — continue with register/bond/allocate.
            warn!("init_staking_config skipped/failed ({e}); continuing");
        }
    }

    let register = ActionBuilder::register_validator(RegisterValidatorParams {
        consensus_pubkey: *validator.public_key(),
    })
    .map_err(|e| e.to_string())?;
    match submit_ok(
        client,
        validator,
        validator.address(),
        None,
        vec![register],
    )
    .await
    {
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
    submit_ok(
        client,
        validator,
        validator.address(),
        None,
        vec![allocate],
    )
    .await
    .map_err(|e| format!("allocate_stake: {e}"))?;
    info!("Allocated stake to Committee purpose");
    info!("Committee will become running after prom_pending + prom_running at epoch boundary");
    Ok(lpl)
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    if cli.positions == 0 {
        return Err("--positions must be >= 1".into());
    }
    if cli.total_positions == 0 {
        return Err("--total-positions must be >= 1".into());
    }
    if cli.num_markets == 0 {
        return Err("--num-markets must be >= 1".into());
    }
    if cli.senders == 0 {
        return Err("--senders must be >= 1".into());
    }
    if cli.rate_per_task == 0 {
        return Err("--rate-per-task must be >= 1".into());
    }
    if cli.duration == 0 {
        return Err("--duration must be >= 1".into());
    }
    if cli.positions > 500 {
        warn!(
            "--positions {} exceeds MAX_LIQS_PER_BLOCK=500; epilogue clearinghouse will cap at 500/block",
            cli.positions
        );
    }
    let total_positions = cli.total_positions;
    let position_markets = POSITION_MARKETS.max(1).min(total_positions);
    let positions_per_market = (total_positions + position_markets - 1) / position_markets;

    let rpc = format!("http://{}:26300", cli.address);
    let mempool = format!("{}:26000", cli.address);
    info!("LightPool Clearinghouse Liquidate Burst");
    info!("========================================");
    info!("RPC: {}", rpc);
    info!("Mempool: {}", mempool);
    info!(
        "Positions: {total_positions} on {position_markets} markets (~{positions_per_market}/mkt); \
         liq ~{}/block via ~{} ora/block",
        cli.positions, LIQ_MARKETS
    );
    info!(
        "Phase 6 burst: markets={} senders={} rate={}/s duration={}s (1 mempool connection)",
        cli.num_markets, cli.senders, cli.rate_per_task, cli.duration
    );
    info!("no_liquidate: {}", cli.no_liquidate);
    info!("skip_staking: {}", cli.skip_staking);

    // Health check
    let client = LightPoolClient::new(&rpc).with_timeout(Duration::from_secs(60));
    match client.health_check().await {
        Ok(true) => info!("Node healthy"),
        Ok(false) => return Err("Node responded but not healthy".into()),
        Err(e) => return Err(format!("health check failed: {e}")),
    }

    let mempool_rate = Arc::new(AtomicU64::new(SETUP_BURST_RATE));
    let (mempool_out, mempool_worker) = spawn_mempool_worker(mempool.clone(), Arc::clone(&mempool_rate));

    let lender = Arc::new(load_default_wallet_signer()?);
    let seller = Arc::new(Signer::new());
    let bidder = Arc::new(Signer::new());
    info!("Lender (default wallet): {}", lender.address());
    info!("Seller: {}", seller.address());
    info!("Bidder: {}", bidder.address());

    // --- Phase 1: committee ---
    if !cli.skip_staking {
        info!("Phase 1: promote committee (staking init/register/bond/allocate)");
        setup_staking_committee(&client, lender.as_ref()).await?;
    } else {
        info!("Phase 1: skipping staking (--skip-staking)");
    }

    // --- Phase 2: tokens & pools ---
    info!("Phase 2: creating USDT / BTC / {} margin pools ...", POOLS);
    let pool_supply_total = BORROW_AMOUNT
        .saturating_mul(total_positions as u64)
        .saturating_mul(2)
        .saturating_add(1_000_000 * TOKEN_SCALE);
    let per_pool_supply = pool_supply_total
        .saturating_add(POOLS as u64 - 1)
        / POOLS as u64;
    let usdt = create_token(
        &client,
        lender.as_ref(),
        "USD Tether",
        "USDT",
        per_pool_supply
            .saturating_mul(POOLS as u64)
            .saturating_add(COLLATERAL.saturating_mul(total_positions as u64))
            .saturating_add(
                quote_for_base(TRADE_AMOUNT, TRADE_PRICE).saturating_mul(total_positions as u64)
                    * 2,
            ),
    )
    .await?;
    let btc_supply = TRADE_AMOUNT
        .saturating_mul(total_positions as u64)
        .saturating_mul(2);
    let btc = create_token(&client, seller.as_ref(), "Bitcoin", "BTC", btc_supply).await?;

    mempool_rate.store(SETUP_BURST_RATE, Ordering::Relaxed);
    let pools = setup_pools_mempool_burst(
        &client,
        &mempool_out,
        lender.as_ref(),
        usdt,
        POOLS,
        per_pool_supply,
    )
    .await?;

    // Fund bidder for liquidation bids (buy base during IOC sell).
    let bid_quote_each = quote_for_base(TRADE_AMOUNT, TRADE_PRICE).saturating_mul(2);
    let bid_fund = bid_quote_each.saturating_mul(total_positions as u64);
    transfer(
        &client,
        lender.as_ref(),
        usdt,
        bidder.address(),
        bid_fund,
    )
    .await?;

    // --- Phase 3: spot markets for checkpoint (if needed) + Phase 6 load ---
    info!("Phase 3: setup spot markets + fund senders");
    let spot_fund = spot_fund_amount_per_sender(cli.rate_per_task, cli.duration);
    let checkpoint_fund = checkpoint_fund_amount_per_sender();
    let total_fund = spot_fund.saturating_add(checkpoint_fund);
    let (spot_senders, spot_markets, baseline_latency) = {
        let baseline_latency = measure_baseline_latency(&client, lender.as_ref()).await?;
        let markets = setup_spot_burst_markets(
            &client,
            &mempool_out,
            lender.as_ref(),
            cli.num_markets,
            total_fund,
            cli.senders,
        )
        .await?;
        let senders = build_burst_senders(cli.senders, markets.len());
        fund_burst_senders(
            &mempool_out,
            lender.as_ref(),
            &senders,
            &markets,
            total_fund,
        )
        .await?;
        if let Some(sample) = senders.first() {
            let market = &markets[sample.market_index];
            match measure_place_order_tx_size(sample, market, SPOT_ORDER_AMOUNT) {
                Ok(size) => {
                    info!("Spot place_order tx size: {size} bytes");
                    info!(
                        "Expected bandwidth: {:.2} MB/s",
                        (size as f64 * cli.rate_per_task as f64) / (1024.0 * 1024.0)
                    );
                }
                Err(e) => warn!("Failed to measure spot tx size: {e}"),
            }
        }
        (Arc::new(senders), Arc::new(markets), baseline_latency)
    };

    // --- Phase 4: positions ---
    info!("Phase 4: setup {total_positions} positions");
    let fixtures = setup_positions_mempool_burst(
        &client,
        &mempool_out,
        &lender,
        &seller,
        &bidder,
        usdt,
        btc,
        &pools,
        total_positions,
        BORROWERS,
        position_markets,
    )
    .await?;
    let tip_after_setup = rpc_get_committed_block_num(&rpc).await.unwrap_or(0);
    info!(
        "Phase 4 done: {} positions ready (tip={tip_after_setup})",
        fixtures.len()
    );

    // --- Phase 5: first checkpoint only if not already past ---
    let tip = if tip_after_setup >= EPOCH_LENGTH {
        info!(
            "Phase 5: tip={tip_after_setup} already >= first checkpoint {EPOCH_LENGTH}; skip checkpoint burst"
        );
        tip_after_setup
    } else {
        info!(
            "Phase 5: pass first checkpoint (tip={tip_after_setup}→{EPOCH_LENGTH}, rate={CHECKPOINT_BURST_RATE}/s)"
        );
        drive_past_checkpoint_with_place_orders(
            &rpc,
            mempool_out.clone(),
            Arc::clone(&mempool_rate),
            Arc::clone(&spot_senders),
            Arc::clone(&spot_markets),
            EPOCH_LENGTH,
            CHECKPOINT_TIMEOUT_SECS,
            SPOT_ORDER_AMOUNT,
        )
        .await?
    };
    info!(
        "Phase 5 done: tip={tip}, positions={}",
        fixtures.len()
    );

    // --- Phase 6a: ora_submit on a quiet mempool, then 6b: spot load ---
    let do_liquidate = !cli.no_liquidate;
    let oracle_price = if do_liquidate { CRASH_PRICE } else { TRADE_PRICE };
    let liqs_per_sec = (cli.positions as u64).saturating_mul(BLOCKS_PER_SEC);
    let burst_rate = cli.rate_per_task;
    if do_liquidate {
        info!(
            "Phase 6a: ora_submit crash={oracle_price} first (~{} liq/block, ~{}/s), then 6b spot @ {burst_rate}/s",
            cli.positions, liqs_per_sec
        );
    } else {
        info!(
            "Phase 6a: ora_submit healthy={oracle_price} first, then 6b spot @ {burst_rate}/s (no liquidations)"
        );
    }

    let ws_url = format!("ws://{}:26400", cli.address);
    let liq_ws_stop = Arc::new(AtomicBool::new(false));
    let liq_ws_count = Arc::new(AtomicU64::new(0));
    let liq_ws_handle = {
        let stop = Arc::clone(&liq_ws_stop);
        let counter = Arc::clone(&liq_ws_count);
        let path = cli.liq_log.clone();
        tokio::spawn(async move {
            if let Err(e) = run_liquidation_ws_logger(ws_url, path, stop, counter).await {
                warn!("Liquidation WS logger failed: {e}");
            }
        })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;

    let markets: Arc<Vec<ContractAddress>> = {
        let mut uniq = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for f in &fixtures {
            if seen.insert(f.market) {
                uniq.push(f.market);
            }
        }
        Arc::new(uniq)
    };

    // Quiet window: only ora_submit on the shared connection (no spot competition).
    mempool_rate.store(SETUP_BURST_RATE.max(1), Ordering::Relaxed);
    let tip_before_oracle = rpc_get_committed_block_num(&rpc).await.unwrap_or(tip);
    let oracle_counter = Arc::new(AtomicU64::new(0));
    info!(
        "Phase 6a: queueing ora_submit for {} markets @ price={} (tip={tip_before_oracle})",
        markets.len(),
        oracle_price
    );
    match mempool_oracle_price_burst(
        mempool_out.clone(),
        Arc::clone(&lender),
        Arc::clone(&markets),
        LIQ_MARKETS,
        BLOCKS_PER_SEC,
        cli.duration.max(1),
        oracle_price,
        Arc::clone(&oracle_counter),
    )
    .await
    {
        Ok(()) => {}
        Err(e) => warn!("Oracle mempool burst failed: {e}"),
    }
    info!(
        "Phase 6a: queued {} ora_submit; waiting for liquidation window...",
        oracle_counter.load(Ordering::Relaxed)
    );
    wait_for_liquidation_window(
        &rpc,
        tip_before_oracle,
        Arc::clone(&liq_ws_count),
        40,
        60,
    )
    .await;
    info!(
        "Phase 6a done: liquidations so far = {}",
        liq_ws_count.load(Ordering::Relaxed)
    );

    // Spot load after oracle / CH had a chance to land.
    mempool_rate.store(cli.rate_per_task.max(1), Ordering::Relaxed);
    info!(
        "Phase 6b: spot place_order load: senders={} markets={} @ {}/s",
        spot_senders.len(),
        spot_markets.len(),
        cli.rate_per_task
    );
    let spot_burst_join = spawn_spot_place_order_burst(
        mempool_out.clone(),
        Arc::clone(&spot_senders),
        Arc::clone(&spot_markets),
        cli.duration,
        SPOT_ORDER_AMOUNT,
    );
    let (handle, monitor, counter, start_time) = spot_burst_join;
    finalize_spot_burst_metrics(
        &client,
        &spot_senders,
        &spot_markets,
        SPOT_ORDER_AMOUNT,
        handle,
        monitor,
        counter,
        start_time,
        baseline_latency,
    )
    .await;

    // Stop logger: abort unblocks receiver.recv(); remaining events already flushed on each write.
    liq_ws_stop.store(true, Ordering::Relaxed);
    let _ = liq_ws_handle.await;
    info!(
        "Liquidations logged: {} → {}",
        liq_ws_count.load(Ordering::Relaxed),
        cli.liq_log.display()
    );

    drop(mempool_out);
    let _ = mempool_worker.await;

    Ok(())
}
