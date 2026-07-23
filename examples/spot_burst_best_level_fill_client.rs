// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use lightpool_sdk::{
    ActionBuilder, CreateMarketParams, CreateTokenParams, ExecutionStatus, EventData, EventType,
    LightPoolClient, MarketState, OrderParamsType, OrderSide, PlaceOrderParams, SegmentSize,
    Signer, TimeInForce, TransactionBuilder, TransferParams,
};
use lightpool_sdk::spot_events::MarketCreatedEvent;
use lightpool_sdk::token_events::TokenCreatedEvent;

use bytes::Bytes;
use clap::Parser;
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
const DECIMAL_PRECISION: u64 = 1_000_000;

#[derive(Parser)]
#[clap(
    author,
    version,
    about,
    long_about = "Burst client for best-level spot fill: mixed Sell/Buy over 10s, success via final RPC OrderFilled."
)]
struct Cli {
    #[clap(long, default_value = "127.0.0.1")]
    address: String,

    #[clap(long, default_value = "200")]
    num_markets: usize,

    #[clap(long, default_value = "1000")]
    senders: usize,

    #[clap(short, long, default_value = "8")]
    tasks: usize,

    #[clap(short, long, default_value = "20000")]
    rate_per_task: u64,

    #[clap(short, long, default_value = "10")]
    duration: u64,

    #[clap(long, default_value = "100000")]
    order_amount: u64,
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

fn quote_per_order(order_amount: u64, price: u64) -> u64 {
    ((order_amount as u128 * price as u128) / DECIMAL_PRECISION as u128) as u64
}

fn orders_per_sender_capacity(rate_per_task: u64, duration: u64) -> u64 {
    rate_per_task.saturating_mul(duration).saturating_add(1)
}

fn base_fund_per_sender(rate_per_task: u64, duration: u64, order_amount: u64) -> u64 {
    order_amount.saturating_mul(orders_per_sender_capacity(rate_per_task, duration))
}

fn quote_fund_per_sender(
    rate_per_task: u64,
    duration: u64,
    order_amount: u64,
    ask_price: u64,
) -> u64 {
    quote_per_order(order_amount, ask_price)
        .saturating_mul(orders_per_sender_capacity(rate_per_task, duration))
}

fn senders_for_market(senders: usize, num_markets: usize, market_index: usize) -> usize {
    senders / num_markets + if market_index < senders % num_markets { 1 } else { 0 }
}

fn market_ask_price(market_index: usize) -> u64 {
    let base = 10_000_000u64 + market_index as u64 * 1_000_000;
    base.saturating_add(100 * TICK_SIZE)
}

fn count_order_filled_events(receipt: &lightpool_sdk::TransactionReceipt) -> u64 {
    receipt
        .events
        .iter()
        .filter(|event| {
            matches!(
                &event.event_type,
                EventType::Call(action_name) if action_name == "order_filled"
            )
        })
        .count() as u64
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
    num_tokens: usize,
    base_fund: u64,
    quote_fund_for_market: &[u64],
    senders: usize,
    num_markets: usize,
) -> Result<(Vec<lightpool_sdk::ContractAddress>, Duration), String> {
    info!("Creating {} tokens...", num_tokens);

    let creator_address = creator.address();
    let mut all_tokens = Vec::with_capacity(num_tokens);
    let rpc_start = Instant::now();

    for batch_start in (0..num_tokens).step_by(SETUP_ACTIONS_BATCH) {
        let batch_end = std::cmp::min(batch_start + SETUP_ACTIONS_BATCH, num_tokens);
        let mut actions = Vec::with_capacity(batch_end - batch_start);

        for token_index in batch_start..batch_end {
            let market_index = token_index / 2;
            let is_base = token_index % 2 == 0;
            let market_senders = senders_for_market(senders, num_markets, market_index);
            let total_supply = if is_base {
                base_fund
                    .saturating_mul(market_senders as u64)
                    .saturating_add(base_fund)
            } else {
                let quote_fund = quote_fund_for_market
                    .get(market_index)
                    .copied()
                    .unwrap_or(base_fund);
                quote_fund
                    .saturating_mul(market_senders as u64)
                    .saturating_add(quote_fund)
            };

            let create_params = CreateTokenParams {
                name: format!("BestFillToken{}", token_index + 1).into(),
                symbol: format!("BFT{}", token_index + 1).into(),
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
        info!(
            "Created tokens batch {}-{} ({} total so far)",
            batch_start,
            batch_end - 1,
            all_tokens.len()
        );
    }

    if all_tokens.len() != num_tokens {
        return Err(format!(
            "Expected {} tokens from creation events, got {}",
            num_tokens,
            all_tokens.len()
        ));
    }

    Ok((all_tokens, rpc_start.elapsed()))
}

async fn create_markets(
    client: &LightPoolClient,
    creator: &Signer,
    tokens: &[lightpool_sdk::ContractAddress],
    num_markets: usize,
) -> Result<Vec<SpotMarketInfo>, String> {
    info!("Creating {} markets...", num_markets);

    let creator_address = creator.address();
    let mut all_markets = Vec::with_capacity(num_markets);

    for batch_start in (0..num_markets).step_by(SETUP_ACTIONS_BATCH) {
        let batch_end = std::cmp::min(batch_start + SETUP_ACTIONS_BATCH, num_markets);
        let mut actions = Vec::with_capacity(batch_end - batch_start);

        for market_index in batch_start..batch_end {
            let base_index = market_index * 2;
            let quote_index = base_index + 1;
            if quote_index >= tokens.len() {
                return Err(format!(
                    "Not enough tokens for market {} (need {}, have {})",
                    market_index,
                    quote_index + 1,
                    tokens.len()
                ));
            }

            let market_create_params = CreateMarketParams {
                name: format!("BestFillMarket{}", market_index + 1).into(),
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
            };

            let market_create_action = ActionBuilder::create_market(market_create_params)
                .map_err(|e| format!("Failed to create market action: {}", e))?;
            actions.push(market_create_action);
        }

        let receipt = submit_signed_actions(client, creator, actions).await?;
        all_markets.extend(extract_markets_from_events(&receipt));
        info!(
            "Created markets batch {}-{} ({} total so far)",
            batch_start,
            batch_end - 1,
            all_markets.len()
        );
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

async fn fund_burst_senders(
    client: &LightPoolClient,
    creator: &Signer,
    senders: &[BurstSpotSender],
    markets: &[SpotMarketInfo],
    base_fund: u64,
    quote_fund_for_market: &[u64],
) -> Result<(), String> {
    if senders.is_empty() {
        return Ok(());
    }

    let creator_address = creator.address();

    info!(
        "Funding {} sender accounts (base {} + quote per market) in one transaction...",
        senders.len(),
        base_fund,
    );

    let mut tx_builder = TransactionBuilder::new()
        .sender(creator_address)
        .expiration(u64::MAX);

    for sender in senders {
        let market = &markets[sender.market_index];
        let quote_fund = quote_fund_for_market
            .get(sender.market_index)
            .copied()
            .unwrap_or(base_fund);

        let base_transfer = ActionBuilder::transfer_token(
            market.base_token,
            TransferParams {
                to: sender.address,
                amount: base_fund,
            },
        )
        .map_err(|e| format!("Failed to create base fund transfer action: {}", e))?;
        tx_builder = tx_builder.add_action(base_transfer);

        let quote_transfer = ActionBuilder::transfer_token(
            market.quote_token,
            TransferParams {
                to: sender.address,
                amount: quote_fund,
            },
        )
        .map_err(|e| format!("Failed to create quote fund transfer action: {}", e))?;
        tx_builder = tx_builder.add_action(quote_transfer);
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
            return Err(format!("Fund transaction failed: {}", error_msg));
        }
        return Err("Fund transaction failed".to_string());
    }

    info!("Funded {} sender accounts", senders.len());
    Ok(())
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

async fn burst_mixed_order_task(
    task_id: usize,
    mempool_addr: String,
    senders: Arc<Vec<BurstSpotSender>>,
    markets: Arc<Vec<SpotMarketInfo>>,
    start_index: usize,
    end_index: usize,
    rate_per_second: u64,
    duration_secs: u64,
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

    info!(
        "Task {} connecting to mempool at {} (sender range {}-{}, {} accounts)",
        task_id,
        mempool_addr,
        start_index,
        end_index,
        range_size
    );

    let stream = TcpStream::connect(&mempool_addr)
        .await
        .map_err(|e| format!("Task {}: Failed to connect to mempool: {}", task_id, e))?;

    let mut transport = Framed::new(stream, LengthDelimitedCodec::new());

    let effective_rate = if rate_per_second == 0 { 1 } else { rate_per_second };
    let mut rate_tokens = 0.0f64;
    let mut rate_last_refill = Instant::now();
    let max_burst_tokens = (effective_rate as f64).max(1.0);

    let end_time = Instant::now() + Duration::from_secs(duration_secs);

    let mut tx_count = 0u64;
    let mut expiration = u64::MAX;

    while Instant::now() < end_time {
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

        let is_sell = tx_count % 2 == 0;
        let (side, token_address) = if is_sell {
            (OrderSide::Sell, market.base_token)
        } else {
            (OrderSide::Buy, market.quote_token)
        };

        let order_params = PlaceOrderParams {
            side,
            amount: order_amount,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::GTC,
            },
            limit_price: market.ask_price,
            token_address,
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
        "Task {} completed (sender range {}-{}). Sent {} mixed order transactions",
        task_id,
        start_index,
        end_index,
        tx_count
    );
    Ok(())
}

async fn probe_fill_order_rpc(
    client: &LightPoolClient,
    sender: &BurstSpotSender,
    market: &SpotMarketInfo,
    order_amount: u64,
) -> Result<(bool, u64), String> {
    let order_params = PlaceOrderParams {
        side: OrderSide::Buy,
        amount: order_amount,
        order_type: OrderParamsType::Limit {
            tif: TimeInForce::GTC,
        },
        limit_price: market.ask_price,
        token_address: market.quote_token,
    };

    let place_order_action = ActionBuilder::place_order(market.market_address, order_params)
        .map_err(|e| format!("Failed to create final place order action: {}", e))?;

    let place_order_tx = TransactionBuilder::new()
        .sender(sender.address)
        .expiration(u64::MAX)
        .add_action(place_order_action)
        .build_and_sign_only(sender.signer.as_ref())
        .map_err(|e| format!("Failed to build final transaction: {}", e))?;

    let response = client
        .submit_transaction(place_order_tx)
        .await
        .map_err(|e| format!("Failed to submit final transaction: {}", e))?;

    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(format!("Final transaction failed: {}", error_msg));
        }
        return Err("Final transaction failed".to_string());
    }

    let order_filled_count = count_order_filled_events(&response.receipt);
    Ok((true, order_filled_count))
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

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

    info!("LightPool Best-Level Spot Fill Burst Client");
    info!("============================================");
    info!("Address: {}", cli.address);

    let rpc_addr = format!("http://{}:26300", cli.address);
    let mempool_addr = format!("{}:26000", cli.address);
    let base_fund = base_fund_per_sender(cli.rate_per_task, cli.duration, cli.order_amount);
    let num_tokens = cli.num_markets * 2;

    let quote_fund_for_market: Vec<u64> = (0..cli.num_markets)
        .map(|market_index| {
            quote_fund_per_sender(
                cli.rate_per_task,
                cli.duration,
                cli.order_amount,
                market_ask_price(market_index),
            )
        })
        .collect();

    info!("RPC Address: {}", rpc_addr);
    info!("Mempool Address: {}", mempool_addr);
    info!("Markets: {}", cli.num_markets);
    info!("Tokens: {}", num_tokens);
    info!("Senders: {}", cli.senders);
    info!("Tasks: {}", cli.tasks);
    info!("Rate per task: {} orders/s", cli.rate_per_task);
    info!("Duration: {} seconds", cli.duration);
    info!("Order amount: {}", cli.order_amount);
    info!("Base fund per sender: {}", base_fund);
    info!(
        "Expected total mempool orders: ~{}",
        cli.rate_per_task
            .saturating_mul(cli.duration)
            .saturating_mul(cli.tasks as u64)
    );

    let creator = Arc::new(Signer::new());
    info!("Creator address: {}", creator.address());

    let client = LightPoolClient::new(&rpc_addr).with_timeout(Duration::from_secs(30));

    info!("Testing RPC connection...");
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

    info!("Phase 1: Creating tokens...");
    let (_tokens, baseline_latency) = create_tokens(
        &client,
        creator.as_ref(),
        num_tokens,
        base_fund,
        &quote_fund_for_market,
        cli.senders,
        cli.num_markets,
    )
    .await?;

    info!("Waiting for token creation to be processed...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    info!("Phase 2: Creating markets...");
    let markets = create_markets(&client, creator.as_ref(), &_tokens, cli.num_markets).await?;

    if let Some(first_market) = markets.first() {
        info!("First market address: {}", first_market.market_address);
    }

    info!("Waiting for market creation to be processed...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    let senders = build_burst_senders(cli.senders, cli.num_markets);

    info!("Phase 3: Funding senders...");
    fund_burst_senders(
        &client,
        creator.as_ref(),
        &senders,
        &markets,
        base_fund,
        &quote_fund_for_market,
    )
    .await?;

    info!("Waiting for sender funding to be processed...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    info!(
        "Starting burst mixed Sell/Buy at best ask: {} tasks over {} senders across {} markets...",
        cli.tasks,
        senders.len(),
        markets.len()
    );

    let senders = Arc::new(senders);
    let markets = Arc::new(markets);
    let semaphore = Arc::new(Semaphore::new(cli.tasks));
    let counter = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    let senders_per_task = cli.senders / cli.tasks;
    let remaining_senders = cli.senders % cli.tasks;

    let mut handles = Vec::new();
    for task_id in 0..cli.tasks {
        let start_index =
            task_id * senders_per_task + std::cmp::min(task_id, remaining_senders);
        let end_index = start_index
            + senders_per_task
            + if task_id < remaining_senders { 1 } else { 0 };

        info!(
            "Task {}: assigned sender range {}-{} ({} accounts)",
            task_id,
            start_index,
            end_index,
            end_index - start_index
        );

        let handle = tokio::spawn(burst_mixed_order_task(
            task_id,
            mempool_addr.clone(),
            Arc::clone(&senders),
            Arc::clone(&markets),
            start_index,
            end_index,
            cli.rate_per_task,
            cli.duration,
            cli.order_amount,
            counter.clone(),
            semaphore.clone(),
        ));
        handles.push(handle);
    }

    let monitor_counter = counter.clone();
    let monitor_duration = cli.duration;
    let monitor_handle = tokio::spawn(async move {
        let mut last_count = 0u64;
        let mut last_time = Instant::now();
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        for i in 0..monitor_duration {
            interval.tick().await;

            let current_count = monitor_counter.load(Ordering::Relaxed);
            let current_time = Instant::now();
            let duration = current_time.duration_since(last_time).as_secs_f64();
            let rate = (current_count - last_count) as f64 / duration;

            info!(
                "Progress [{:2}/{}]: {} total orders, {:.1} orders/s",
                i + 1,
                monitor_duration,
                current_count,
                rate
            );

            last_count = current_count;
            last_time = current_time;
        }
    });

    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => error!("Task {} failed: {}", i, e),
            Err(e) => error!("Task {} panicked: {}", i, e),
        }
    }

    monitor_handle.abort();

    let mempool_send_time = start_time.elapsed();
    let total_orders = counter.load(Ordering::Relaxed);
    let mempool_send_rate = total_orders as f64 / mempool_send_time.as_secs_f64();

    info!("Mempool phase completed:");
    info!("   Total orders sent to mempool: {}", total_orders);
    info!("   Mempool send time: {:.2} seconds", mempool_send_time.as_secs_f64());
    info!("   Mempool send rate: {:.1} orders/s", mempool_send_rate);

    info!("Sending final RPC Buy to verify OrderFilled...");
    let final_sender = &senders[0];
    let final_market = &markets[final_sender.market_index];

    let (final_tx_success, order_filled_count) = match probe_fill_order_rpc(
        &client,
        final_sender,
        final_market,
        cli.order_amount,
    )
    .await
    {
        Ok((success, filled)) => {
            if filled > 0 {
                info!(
                    "Final RPC Buy succeeded with {} order_filled event(s)",
                    filled
                );
            } else {
                warn!("Final RPC Buy succeeded but no order_filled events in receipt");
            }
            (success, filled)
        }
        Err(e) => {
            warn!("Final transaction failed: {}", e);
            (false, 0)
        }
    };

    let test_passed = final_tx_success && order_filled_count > 0;
    let actual_completion_time = start_time.elapsed();
    let actual_throughput = total_orders as f64 / actual_completion_time.as_secs_f64();

    info!("Best-level spot fill burst test completed!");
    info!("==========================================");
    info!(
        "Test result: {}",
        if test_passed {
            "PASS (final tx success + OrderFilled)"
        } else {
            "FAIL"
        }
    );
    info!(
        "Final transaction status: {}",
        if final_tx_success { "SUCCESS" } else { "FAILED" }
    );
    info!("Final order_filled events: {}", order_filled_count);
    info!("Markets: {}", cli.num_markets);
    info!("Total orders sent: {}", total_orders);
    info!(
        "Actual completion time: {:.2} seconds",
        actual_completion_time.as_secs_f64()
    );
    info!("Actual orders per second: {:.1} orders/s", actual_throughput);
    info!("Baseline setup latency: {:.3} seconds", baseline_latency.as_secs_f64());

    if !test_passed {
        return Err(
            "Test failed: final RPC transaction must succeed and emit at least one order_filled event"
                .to_string(),
        );
    }

    Ok(())
}
