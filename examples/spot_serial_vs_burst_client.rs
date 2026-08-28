// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use lightpool_sdk::{
    ActionBuilder, CreateMarketParams, CreateTokenParams, ExecutionStatus, EventData, EventType,
    LightPoolClient, OrderParamsType, OrderSide, PlaceOrderParams, Signer, TimeInForce,
    TransactionBuilder, TransferParams, MarketState, SegmentSize,
};
use lightpool_sdk::spot_events::MarketCreatedEvent;
use lightpool_sdk::token_events::TokenCreatedEvent;

use bytes::Bytes;
use clap::{Parser, ValueEnum};
use env_logger::Env;
use futures::sink::SinkExt;
use log::{error, info, warn};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

const TICK_SIZE: u64 = 100_000;
const MIN_ORDER_SIZE: u64 = 100_000;
const SETUP_ACTIONS_BATCH: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum RunMode {
    /// One market, one sender, sequential RPC place orders
    Serial,
    /// Multi-market mempool burst (like burst_spot)
    Burst,
    /// Run serial first, then burst, and print comparison
    Both,
}

#[derive(Parser)]
#[clap(
    author,
    version,
    about,
    long_about = "Compare serial single-market spot orders vs multi-market mempool burst to reproduce order-node bugs."
)]
struct Cli {
    #[clap(long, default_value = "127.0.0.1")]
    address: String,

  /// serial | burst | both
    #[clap(long, value_enum, default_value_t = RunMode::Both)]
    mode: RunMode,

    /// Orders placed per market (4800 flushes ~160 external order nodes)
    #[clap(long, default_value = "4800")]
    orders_per_market: u64,

    #[clap(long, default_value = "200")]
    num_markets: usize,

    #[clap(long, default_value = "1000")]
    senders: usize,

    #[clap(short, long, default_value = "8")]
    tasks: usize,

    #[clap(short, long, default_value = "20000")]
    rate_per_task: u64,

    #[clap(long, default_value = "100000")]
    order_amount: u64,

    /// Prefix for token/market names so serial and burst setups do not collide on one node
    #[clap(long, default_value = "Compare")]
    setup_prefix: String,
}

#[derive(Debug, Clone)]
struct SpotMarketInfo {
    market_address: lightpool_sdk::ContractAddress,
    base_token: lightpool_sdk::ContractAddress,
    quote_token: lightpool_sdk::ContractAddress,
    ask_price: u64,
}

struct BurstSpotSender {
    signer: Arc<Signer>,
    address: lightpool_sdk::Address,
    market_index: usize,
}

#[derive(Debug, Clone)]
struct RunResult {
    label: String,
    orders_attempted: u64,
    orders_succeeded: u64,
    orders_failed: u64,
    order_node_errors: u64,
    sample_errors: Vec<String>,
    elapsed: Duration,
    probe_success: bool,
}

fn senders_for_market(senders: usize, num_markets: usize, market_index: usize) -> usize {
    senders / num_markets + if market_index < senders % num_markets { 1 } else { 0 }
}

fn market_ask_price(market_index: usize) -> u64 {
    let base = 10_000_000u64 + market_index as u64 * 1_000_000;
    base.saturating_add(100 * TICK_SIZE)
}

fn fund_amount_per_sender(orders_per_market: u64, order_amount: u64) -> u64 {
    order_amount
        .saturating_mul(orders_per_market)
        .saturating_add(order_amount)
}

fn extract_token_addresses_from_events(
    receipt: &lightpool_sdk::TransactionReceipt,
) -> Vec<lightpool_sdk::ContractAddress> {
    let mut tokens = Vec::new();
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "token_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(token_created_event) = bincode::deserialize::<TokenCreatedEvent>(data)
                    {
                        tokens.push(token_created_event.token_address);
                    }
                }
            }
        }
    }
    tokens
}

fn extract_markets_from_events(
    receipt: &lightpool_sdk::TransactionReceipt,
) -> Vec<SpotMarketInfo> {
    let mut markets = Vec::new();
    for event in &receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "market_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(market_created_event) =
                        bincode::deserialize::<MarketCreatedEvent>(data)
                    {
                        let market_index = markets.len();
                        markets.push(SpotMarketInfo {
                            market_address: market_created_event.market_address,
                            base_token: market_created_event.base_token,
                            quote_token: market_created_event.quote_token,
                            ask_price: market_ask_price(market_index),
                        });
                    }
                }
            }
        }
    }
    markets
}

async fn submit_signed_actions(
    client: &LightPoolClient,
    creator: &Signer,
    actions: Vec<lightpool_sdk::Action>,
) -> Result<lightpool_sdk::TransactionReceipt, String> {
    if actions.is_empty() {
        return Err("Cannot submit an empty action batch".to_string());
    }

    let creator_address = creator.address();
    let mut tx_builder = TransactionBuilder::new()
        .sender(creator_address)
        .expiration(u64::MAX);

    for action in actions {
        tx_builder = tx_builder.add_action(action);
    }

    let tx = tx_builder
        .build_and_sign_only(creator)
        .map_err(|e| format!("Failed to build transaction: {}", e))?;

    let response = client
        .submit_transaction(tx)
        .await
        .map_err(|e| format!("Failed to submit transaction: {}", e))?;

    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(format!("Transaction failed: {}", error_msg));
        }
        return Err("Transaction failed".to_string());
    }

    Ok(response.receipt)
}

async fn create_tokens(
    client: &LightPoolClient,
    creator: &Signer,
    prefix: &str,
    num_tokens: usize,
    fund_amount: u64,
    senders: usize,
    num_markets: usize,
) -> Result<Vec<lightpool_sdk::ContractAddress>, String> {
    info!("Creating {} tokens (prefix={})...", num_tokens, prefix);

    let creator_address = creator.address();
    let mut all_tokens = Vec::with_capacity(num_tokens);

    for batch_start in (0..num_tokens).step_by(SETUP_ACTIONS_BATCH) {
        let batch_end = std::cmp::min(batch_start + SETUP_ACTIONS_BATCH, num_tokens);
        let mut actions = Vec::with_capacity(batch_end - batch_start);

        for token_index in batch_start..batch_end {
            let market_index = token_index / 2;
            let is_base = token_index % 2 == 0;
            let total_supply = if is_base {
                let market_senders = senders_for_market(senders, num_markets, market_index);
                fund_amount
                    .saturating_mul(market_senders as u64)
                    .saturating_add(fund_amount)
            } else {
                fund_amount
            };

            let create_params = CreateTokenParams {
                name: format!("{}Token{}", prefix, token_index + 1).into(),
                symbol: format!("{}T{}", prefix, token_index + 1).into(),
                total_supply,
                mintable: false,
                to: creator_address,
            };

            let create_action = ActionBuilder::create_token(create_params)
                .map_err(|e| format!("Failed to create token action: {}", e))?;
            actions.push(create_action);
        }

        let receipt = submit_signed_actions(client, creator, actions).await?;
        all_tokens.extend(extract_token_addresses_from_events(&receipt));
    }

    if all_tokens.len() != num_tokens {
        return Err(format!(
            "Expected {} tokens from creation events, got {}",
            num_tokens,
            all_tokens.len()
        ));
    }

    Ok(all_tokens)
}

async fn create_markets(
    client: &LightPoolClient,
    creator: &Signer,
    prefix: &str,
    tokens: &[lightpool_sdk::ContractAddress],
    num_markets: usize,
) -> Result<Vec<SpotMarketInfo>, String> {
    info!("Creating {} markets (prefix={})...", num_markets, prefix);

    let creator_address = creator.address();
    let mut all_markets = Vec::with_capacity(num_markets);

    for batch_start in (0..num_markets).step_by(SETUP_ACTIONS_BATCH) {
        let batch_end = std::cmp::min(batch_start + SETUP_ACTIONS_BATCH, num_markets);
        let mut actions = Vec::with_capacity(batch_end - batch_start);

        for market_index in batch_start..batch_end {
            let base_index = market_index * 2;
            let quote_index = base_index + 1;

            let market_create_params = CreateMarketParams {
                name: format!("{}Market{}", prefix, market_index + 1).into(),
                base_token: tokens[base_index],
                quote_token: tokens[quote_index],
                min_order_size: MIN_ORDER_SIZE,
                tick_size: TICK_SIZE,
                maker_fee_bps: 10,
                taker_fee_bps: 20,
                allow_market_orders: true,
                state: MarketState::Active,
                limit_order: true,
                side_book_size: SegmentSize::Large,
                creator: creator_address,
            access: Default::default(),
            };

            let market_create_action = ActionBuilder::create_market(market_create_params)
                .map_err(|e| format!("Failed to create market action: {}", e))?;
            actions.push(market_create_action);
        }

        let receipt = submit_signed_actions(client, creator, actions).await?;
        all_markets.extend(extract_markets_from_events(&receipt));
    }

    if all_markets.len() != num_markets {
        return Err(format!(
            "Expected {} markets from creation events, got {}",
            num_markets,
            all_markets.len()
        ));
    }

    Ok(all_markets)
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

async fn fund_senders(
    client: &LightPoolClient,
    creator: &Signer,
    senders: &[BurstSpotSender],
    markets: &[SpotMarketInfo],
    fund_amount: u64,
) -> Result<(), String> {
    if senders.is_empty() {
        return Ok(());
    }

    let creator_address = creator.address();
    let mut transfer_actions = Vec::with_capacity(senders.len());

    for sender in senders {
        let market = &markets[sender.market_index];
        let transfer_params = TransferParams {
            to: sender.address,
            amount: fund_amount,
        };
        let transfer_action = ActionBuilder::transfer_token(market.base_token, transfer_params)
            .map_err(|e| format!("Failed to create fund transfer action: {}", e))?;
        transfer_actions.push(transfer_action);
    }

    for (batch_id, chunk) in transfer_actions.chunks(SETUP_ACTIONS_BATCH).enumerate() {
        let mut tx_builder = TransactionBuilder::new()
            .sender(creator_address)
            .expiration(u64::MAX);

        for action in chunk {
            tx_builder = tx_builder.add_action(action.clone());
        }

        let fund_tx = tx_builder
            .build_and_sign_only(creator)
            .map_err(|e| format!("Failed to build fund transaction: {}", e))?;

        let response = client
            .submit_transaction(fund_tx)
            .await
            .map_err(|e| format!("Failed to submit fund transaction: {}", e))?;

        if !response.receipt.is_success() {
            if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
                return Err(format!("Fund transaction batch {} failed: {}", batch_id, error_msg));
            }
            return Err(format!("Fund transaction batch {} failed", batch_id));
        }
    }

    Ok(())
}

async fn setup_world(
    client: &LightPoolClient,
    creator: &Signer,
    prefix: &str,
    num_markets: usize,
    num_senders: usize,
    fund_amount: u64,
) -> Result<(Vec<SpotMarketInfo>, Vec<BurstSpotSender>), String> {
    let num_tokens = num_markets * 2;
    let tokens = create_tokens(
        client,
        creator,
        prefix,
        num_tokens,
        fund_amount,
        num_senders,
        num_markets,
    )
    .await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let markets = create_markets(client, creator, prefix, &tokens, num_markets).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let senders = build_burst_senders(num_senders, num_markets);
    fund_senders(client, creator, &senders, &markets, fund_amount).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok((markets, senders))
}

async fn probe_place_order_rpc(
    client: &LightPoolClient,
    sender: &Signer,
    market: &SpotMarketInfo,
    order_amount: u64,
) -> Result<(), String> {
    let order_params = PlaceOrderParams {
        side: OrderSide::Sell,
        amount: order_amount,
        order_type: OrderParamsType::Limit {
            tif: TimeInForce::GTC,
        },
        limit_price: market.ask_price,
        token_address: market.base_token,
    };

    let place_order_action = ActionBuilder::place_order(market.market_address, order_params)
        .map_err(|e| format!("Failed to create probe place order action: {}", e))?;

    let tx = TransactionBuilder::new()
        .sender(sender.address())
        .expiration(u64::MAX)
        .add_action(place_order_action)
        .build_and_sign_only(sender)
        .map_err(|e| format!("Failed to build probe transaction: {}", e))?;

    let response = client
        .submit_transaction(tx)
        .await
        .map_err(|e| format!("Failed to submit probe transaction: {}", e))?;

    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(error_msg.clone());
        }
        return Err("Probe transaction failed".to_string());
    }

    Ok(())
}

async fn run_serial(
    client: &LightPoolClient,
    prefix: &str,
    orders_per_market: u64,
    order_amount: u64,
) -> Result<RunResult, String> {
    info!("=== SERIAL PHASE (1 market, 1 sender, RPC) ===");

    let creator = Signer::new();
    let fund_amount = fund_amount_per_sender(orders_per_market, order_amount);
    let (markets, senders) = setup_world(client, &creator, prefix, 1, 1, fund_amount).await?;

    let market = &markets[0];
    let sender = &senders[0];

    let start = Instant::now();
    let mut succeeded = 0u64;
    let mut failed = 0u64;
    let mut order_node_errors = 0u64;
    let mut sample_errors = Vec::new();
    let mut expiration = u64::MAX;

    for order_index in 0..orders_per_market {
        let order_params = PlaceOrderParams {
            side: OrderSide::Sell,
            amount: order_amount,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::GTC,
            },
            limit_price: market.ask_price,
            token_address: market.base_token,
        };

        let place_order_action = ActionBuilder::place_order(market.market_address, order_params)
            .map_err(|e| format!("Failed to create place order action: {}", e))?;

        let tx = TransactionBuilder::new()
            .sender(sender.address)
            .expiration(expiration)
            .add_action(place_order_action)
            .build_and_sign_only(sender.signer.as_ref())
            .map_err(|e| format!("Failed to build transaction: {}", e))?;

        expiration = expiration.saturating_sub(1);

        match client.submit_transaction(tx).await {
            Ok(response) => {
                if response.receipt.is_success() {
                    succeeded += 1;
                } else {
                    failed += 1;
                    if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
                        if error_msg.contains("Order node") {
                            order_node_errors += 1;
                        }
                        if sample_errors.len() < 5 {
                            sample_errors.push(error_msg.clone());
                        }
                    }
                }
            }
            Err(e) => {
                failed += 1;
                let message = e.to_string();
                if message.contains("Order node") {
                    order_node_errors += 1;
                }
                if sample_errors.len() < 5 {
                    sample_errors.push(message);
                }
            }
        }

        if (order_index + 1) % 500 == 0 {
            info!(
                "Serial progress: {}/{} (ok={}, fail={})",
                order_index + 1,
                orders_per_market,
                succeeded,
                failed
            );
        }
    }

    let probe_success = probe_place_order_rpc(client, sender.signer.as_ref(), market, order_amount)
        .await
        .is_ok();

    Ok(RunResult {
        label: format!("serial_{}", prefix),
        orders_attempted: orders_per_market,
        orders_succeeded: succeeded,
        orders_failed: failed,
        order_node_errors,
        sample_errors,
        elapsed: start.elapsed(),
        probe_success,
    })
}

async fn burst_place_order_task(
    task_id: usize,
    mempool_addr: String,
    senders: Arc<Vec<BurstSpotSender>>,
    markets: Arc<Vec<SpotMarketInfo>>,
    start_index: usize,
    end_index: usize,
    target_orders: u64,
    rate_per_second: u64,
    order_amount: u64,
    counter: Arc<AtomicU64>,
    semaphore: Arc<Semaphore>,
) -> Result<(), String> {
    let _permit = semaphore
        .acquire()
        .await
        .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;

    let range_size = end_index - start_index;
    if range_size == 0 {
        return Err(format!(
            "Task {}: empty sender range {}-{}",
            task_id, start_index, end_index
        ));
    }

    let stream = TcpStream::connect(&mempool_addr)
        .await
        .map_err(|e| format!("Task {}: Failed to connect to mempool: {}", task_id, e))?;

    let mut transport = Framed::new(stream, LengthDelimitedCodec::new());

    let effective_rate = if rate_per_second == 0 {
        1
    } else {
        rate_per_second
    };
    let mut rate_tokens = 0.0f64;
    let mut rate_last_refill = Instant::now();
    let max_burst_tokens = (effective_rate as f64).max(1.0);

    let mut tx_count = 0u64;
    let mut expiration = u64::MAX;

    while counter.load(Ordering::Relaxed) < target_orders {
        let now = Instant::now();
        let refill_elapsed = now.duration_since(rate_last_refill).as_secs_f64();
        rate_tokens =
            (rate_tokens + refill_elapsed * effective_rate as f64).min(max_burst_tokens);
        rate_last_refill = now;

        if rate_tokens < 1.0 {
            tokio::time::sleep(Duration::from_millis(1)).await;
            continue;
        }
        rate_tokens -= 1.0;

        let sender_index = start_index + (tx_count as usize % range_size);
        let sender = &senders[sender_index];
        let market = &markets[sender.market_index];

        let order_params = PlaceOrderParams {
            side: OrderSide::Sell,
            amount: order_amount,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::GTC,
            },
            limit_price: market.ask_price,
            token_address: market.base_token,
        };

        let place_order_action = ActionBuilder::place_order(market.market_address, order_params)
            .map_err(|e| format!("Task {}: Failed to create place order action: {}", task_id, e))?;

        let place_order_tx = TransactionBuilder::new()
            .sender(sender.address)
            .expiration(expiration)
            .add_action(place_order_action)
            .build_and_without_sign()
            .map_err(|e| format!("Task {}: Failed to build transaction: {}", task_id, e))?;

        let tx_bytes = bincode::serialize(&place_order_tx)
            .map_err(|e| format!("Task {}: Failed to serialize transaction: {}", task_id, e))?;

        if let Err(e) = transport.send(Bytes::from(tx_bytes)).await {
            warn!("Task {}: Failed to send place order transaction: {}", task_id, e);
            break;
        }

        tx_count += 1;
        expiration = expiration.saturating_sub(1);
        counter.fetch_add(1, Ordering::Relaxed);
    }

    info!(
        "Task {} completed (sender range {}-{}). Sent {} mempool transactions",
        task_id,
        start_index,
        end_index,
        tx_count
    );
    Ok(())
}

async fn run_burst(
    client: &LightPoolClient,
    mempool_addr: &str,
    prefix: &str,
    cli: &Cli,
) -> Result<RunResult, String> {
    info!("=== BURST PHASE (multi-market mempool) ===");

    let fund_amount = fund_amount_per_sender(cli.orders_per_market, cli.order_amount);
    let creator = Signer::new();
    let (markets, senders) = setup_world(
        client,
        &creator,
        prefix,
        cli.num_markets,
        cli.senders,
        fund_amount,
    )
    .await?;

    let target_orders = cli.orders_per_market.saturating_mul(cli.num_markets as u64);
    let senders = Arc::new(senders);
    let markets = Arc::new(markets);
    let semaphore = Arc::new(Semaphore::new(cli.tasks));
    let counter = Arc::new(AtomicU64::new(0));
    let start = Instant::now();

    let senders_per_task = cli.senders / cli.tasks;
    let remaining_senders = cli.senders % cli.tasks;

    let mut handles = Vec::new();
    for task_id in 0..cli.tasks {
        let start_index =
            task_id * senders_per_task + std::cmp::min(task_id, remaining_senders);
        let end_index = start_index
            + senders_per_task
            + if task_id < remaining_senders { 1 } else { 0 };

        let handle = tokio::spawn(burst_place_order_task(
            task_id,
            mempool_addr.to_string(),
            Arc::clone(&senders),
            Arc::clone(&markets),
            start_index,
            end_index,
            target_orders,
            cli.rate_per_task,
            cli.order_amount,
            counter.clone(),
            semaphore.clone(),
        ));
        handles.push(handle);
    }

    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => error!("Burst task {} failed: {}", i, e),
            Err(e) => error!("Burst task {} panicked: {}", i, e),
        }
    }

    let mempool_orders = counter.load(Ordering::Relaxed);
    info!(
        "Mempool submitted {} orders (target {})",
        mempool_orders, target_orders
    );

    tokio::time::sleep(Duration::from_secs(2)).await;

    let probe_sender = &senders[0];
    let probe_market = &markets[probe_sender.market_index];
    let probe_result =
        probe_place_order_rpc(client, probe_sender.signer.as_ref(), probe_market, cli.order_amount)
            .await;

    let mut sample_errors = Vec::new();
    let mut order_node_errors = 0u64;
    if let Err(message) = &probe_result {
        if message.contains("Order node") {
            order_node_errors = 1;
        }
        sample_errors.push(message.clone());
    }

    Ok(RunResult {
        label: format!("burst_{}", prefix),
        orders_attempted: mempool_orders,
        orders_succeeded: mempool_orders,
        orders_failed: 0,
        order_node_errors,
        sample_errors,
        elapsed: start.elapsed(),
        probe_success: probe_result.is_ok(),
    })
}

fn print_run_result(result: &RunResult) {
    info!("--- {} ---", result.label);
    info!("orders_attempted: {}", result.orders_attempted);
    info!("orders_succeeded: {}", result.orders_succeeded);
    info!("orders_failed: {}", result.orders_failed);
    info!("order_node_errors: {}", result.order_node_errors);
    info!("probe_rpc_success: {}", result.probe_success);
    info!("elapsed: {:.2}s", result.elapsed.as_secs_f64());
    if !result.sample_errors.is_empty() {
        info!("sample errors:");
        for err in &result.sample_errors {
            info!("  {}", err);
        }
    }
}

fn print_comparison(serial: &RunResult, burst: &RunResult) {
    info!("=== COMPARISON ===");

    let serial_failed = serial.orders_failed > 0 || !serial.probe_success;
    let burst_failed = !burst.probe_success || burst.order_node_errors > 0;

    if !serial_failed && burst_failed {
        info!("RESULT: burst fails while serial succeeds (hypothesis reproduced)");
    } else if serial_failed && !burst_failed {
        info!("RESULT: serial fails while burst succeeds");
    } else if !serial_failed && !burst_failed {
        info!("RESULT: both serial and burst succeeded on this run");
    } else {
        info!("RESULT: both serial and burst showed failures");
    }

    info!(
        "serial: failed_tx={}, probe_ok={}, node_errors={}",
        serial.orders_failed, serial.probe_success, serial.order_node_errors
    );
    info!(
        "burst: mempool_orders={}, probe_ok={}, node_errors={}",
        burst.orders_attempted, burst.probe_success, burst.order_node_errors
    );
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    if cli.num_markets == 0 {
        return Err("--num-markets must be at least 1".to_string());
    }
    if cli.senders == 0 {
        return Err("--senders must be at least 1".to_string());
    }
    if cli.tasks == 0 {
        return Err("--tasks must be at least 1".to_string());
    }
    if cli.tasks > cli.senders {
        return Err(format!(
            "--tasks ({}) cannot exceed --senders ({})",
            cli.tasks, cli.senders
        ));
    }
    if cli.order_amount < MIN_ORDER_SIZE {
        return Err(format!(
            "--order-amount ({}) must be at least min_order_size ({})",
            cli.order_amount, MIN_ORDER_SIZE
        ));
    }
    if cli.mode != RunMode::Serial && cli.senders % cli.num_markets != 0 {
        warn!(
            "senders ({}) not divisible by num_markets ({}); per-market order counts may be uneven",
            cli.senders, cli.num_markets
        );
    }

    let rpc_addr = format!("http://{}:26300", cli.address);
    let mempool_addr = format!("{}:26000", cli.address);

    info!("Spot Serial vs Burst Compare Client");
    info!("====================================");
    info!("mode: {:?}", cli.mode);
    info!("orders_per_market: {}", cli.orders_per_market);
    info!("num_markets (burst): {}", cli.num_markets);
    info!("senders (burst): {}", cli.senders);
    info!("tasks (burst): {}", cli.tasks);
    info!("rate_per_task (burst): {}", cli.rate_per_task);

    let client = LightPoolClient::new(&rpc_addr).with_timeout(Duration::from_secs(60));

    match client.health_check().await {
        Ok(true) => info!("RPC node is healthy"),
        Ok(false) => {
            error!("RPC node responded but not healthy");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to connect to RPC node: {}", e);
            return Ok(());
        }
    }

    match cli.mode {
        RunMode::Serial => {
            let serial = run_serial(
                &client,
                &format!("{}Serial", cli.setup_prefix),
                cli.orders_per_market,
                cli.order_amount,
            )
            .await?;
            print_run_result(&serial);
        }
        RunMode::Burst => {
            let burst = run_burst(
                &client,
                &mempool_addr,
                &format!("{}Burst", cli.setup_prefix),
                &cli,
            )
            .await?;
            print_run_result(&burst);
        }
        RunMode::Both => {
            let serial = run_serial(
                &client,
                &format!("{}Serial", cli.setup_prefix),
                cli.orders_per_market,
                cli.order_amount,
            )
            .await?;
            print_run_result(&serial);

            let burst = run_burst(
                &client,
                &mempool_addr,
                &format!("{}Burst", cli.setup_prefix),
                &cli,
            )
            .await?;
            print_run_result(&burst);

            print_comparison(&serial, &burst);
        }
    }

    Ok(())
}
