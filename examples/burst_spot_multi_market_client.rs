// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use lightpool_sdk::{
    LightPoolClient, TransactionBuilder, ActionBuilder, Signer,
    Address, ContractAddress, CreateTokenParams, CreateMarketParams, PlaceOrderParams,
    TransferParams, ExecutionStatus, EventType, EventData,
    OrderSide, TimeInForce, OrderParamsType, MarketState, SegmentSize,
};
use lightpool_sdk::spot_events::MarketCreatedEvent;
use lightpool_sdk::token_events::TokenCreatedEvent;

use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Semaphore;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::sink::SinkExt;
use bytes::Bytes;
use clap::Parser;
use env_logger::Env;
use log::{info, warn, error};

const TICK_SIZE: u64 = 100_000;
const MIN_ORDER_SIZE: u64 = 100_000;
const SETUP_ACTIONS_BATCH: usize = 64;

#[derive(Parser)]
#[clap(
    author,
    version,
    about,
    long_about = "Burst client for multi-market LightPool spot place_order transactions."
)]
struct Cli {
    /// The base address of the node (will generate RPC and mempool addresses from this)
    #[clap(long, default_value = "127.0.0.1")]
    address: String,

    /// Number of spot markets to create
    #[clap(long, default_value = "500")]
    num_markets: usize,

    /// Number of sender accounts to fund for parallel burst orders
    #[clap(long, default_value = "1024")]
    senders: usize,

    /// Number of concurrent burst tasks
    #[clap(short, long, default_value = "8")]
    tasks: usize,

    /// Place-order rate per task (transactions per second)
    #[clap(short, long, default_value = "500")]
    rate_per_task: u64,

    /// Duration to run burst trading (seconds)
    #[clap(short, long, default_value = "10")]
    duration: u64,

    /// Order amount per transaction (smallest units)
    #[clap(long, default_value = "100000")]
    order_amount: u64,
}

#[derive(Debug, Clone)]
struct SpotMarketInfo {
    market_address: ContractAddress,
    base_token: ContractAddress,
    quote_token: ContractAddress,
    ask_price: u64,
}

struct BurstSpotSender {
    signer: Arc<Signer>,
    address: Address,
    market_index: usize,
}

fn fund_amount_per_sender(cli: &Cli) -> u64 {
    cli.order_amount
        .saturating_mul(cli.rate_per_task)
        .saturating_mul(cli.duration)
        .saturating_add(cli.order_amount)
}

fn senders_for_market(senders: usize, num_markets: usize, market_index: usize) -> usize {
    senders / num_markets + if market_index < senders % num_markets { 1 } else { 0 }
}

fn market_ask_price(market_index: usize) -> u64 {
    let base = 10_000_000u64 + market_index as u64 * 1_000_000;
    base.saturating_add(100 * TICK_SIZE)
}

fn extract_token_addresses_from_events(
    receipt: &lightpool_sdk::TransactionReceipt,
) -> Vec<ContractAddress> {
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

    let response = client.submit_transaction(tx).await
        .map_err(|e| format!("Failed to submit transaction: {}", e))?;

    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(format!("Transaction failed: {}", error_msg));
        }
        return Err("Transaction failed".to_string());
    }

    Ok(response.receipt)
}

async fn measure_create_token_latency(
    client: &LightPoolClient,
    creator: &Signer,
) -> Result<Duration, String> {
    info!("Measuring baseline processing latency with token creation...");

    let creator_address = creator.address();
    let create_params = CreateTokenParams {
        name: "BaselineLatency".into(),
        symbol: "BASE".into(),
        total_supply: 1,
        mintable: false,
        to: creator_address,
    };

    let create_action = ActionBuilder::create_token(create_params)
        .map_err(|e| format!("Failed to create baseline token action: {}", e))?;

    let create_tx = TransactionBuilder::new()
        .sender(creator_address)
        .expiration(u64::MAX)
        .add_action(create_action)
        .build_and_sign_only(creator)
        .map_err(|e| format!("Failed to build baseline token transaction: {}", e))?;

    let rpc_start = Instant::now();
    let response = client
        .submit_transaction(create_tx)
        .await
        .map_err(|e| format!("Failed to submit baseline token transaction: {}", e))?;
    let rpc_latency = rpc_start.elapsed();

    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(format!("Baseline token creation failed: {}", error_msg));
        }
        return Err("Baseline token creation failed".to_string());
    }

    Ok(rpc_latency)
}

async fn create_tokens(
    client: &LightPoolClient,
    creator: &Signer,
    num_tokens: usize,
    fund_amount: u64,
    senders: usize,
    num_markets: usize,
) -> Result<Vec<ContractAddress>, String> {
    info!("Creating {} tokens...", num_tokens);

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
                name: format!("BurstToken{}", token_index + 1).into(),
                symbol: format!("BT{}", token_index + 1).into(),
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

    Ok(all_tokens)
}

async fn create_markets(
    client: &LightPoolClient,
    creator: &Signer,
    tokens: &[ContractAddress],
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
                name: format!("BurstMarket{}", market_index + 1).into(),
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
        let transfer_action = ActionBuilder::transfer_token(
            market.base_token,
            transfer_params,
        )
        .map_err(|e| format!("Failed to create fund transfer action: {}", e))?;
        transfer_actions.push(transfer_action);
    }

    info!(
        "Funding {} sender accounts with {} base token each...",
        senders.len(),
        fund_amount,
    );

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

        let response = client.submit_transaction(fund_tx).await
            .map_err(|e| format!("Failed to submit fund transaction: {}", e))?;

        if !response.receipt.is_success() {
            if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
                return Err(format!("Fund transaction batch {} failed: {}", batch_id, error_msg));
            }
            return Err(format!("Fund transaction batch {} failed", batch_id));
        }
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

fn measure_place_order_tx_size(
    sender: &BurstSpotSender,
    market: &SpotMarketInfo,
    order_amount: u64,
) -> Result<usize, String> {
    let order_params = PlaceOrderParams {
        side: OrderSide::Sell,
        amount: order_amount,
        order_type: OrderParamsType::Limit {
            tif: TimeInForce::GTC,
        },
        limit_price: market.ask_price,
        token_address: market.base_token,
    };

    let place_order_action = ActionBuilder::place_order(
        market.market_address,
        order_params,
    )
    .map_err(|e| format!("Failed to create place order action: {}", e))?;

    let place_order_tx = TransactionBuilder::new()
        .sender(sender.address)
        .expiration(u64::MAX)
        .add_action(place_order_action)
        .build_and_without_sign()
        .map_err(|e| format!("Failed to build transaction: {}", e))?;

    let tx_bytes = bincode::serialize(&place_order_tx)
        .map_err(|e| format!("Failed to serialize transaction: {}", e))?;

    Ok(tx_bytes.len())
}

async fn burst_place_order_task(
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
    let _permit = semaphore.acquire().await.map_err(|e| format!("Failed to acquire semaphore: {}", e))?;

    let range_size = end_index - start_index;
    if range_size == 0 {
        return Err(format!("Task {}: empty sender range {}-{}", task_id, start_index, end_index));
    }

    info!(
        "Task {} connecting to mempool at {} (sender range {}-{}, {} accounts)",
        task_id, mempool_addr, start_index, end_index, range_size
    );

    let stream = TcpStream::connect(&mempool_addr).await
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
        rate_tokens = (rate_tokens + refill_elapsed * effective_rate as f64).min(max_burst_tokens);
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

        let place_order_action = ActionBuilder::place_order(
            market.market_address,
            order_params,
        )
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
        "Task {} completed (sender range {}-{}). Sent {} place order transactions",
        task_id, start_index, end_index, tx_count
    );
    Ok(())
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

    info!("LightPool Multi-Market Spot Burst Client");
    info!("=========================================");
    info!("Address: {}", cli.address);

    let rpc_addr = format!("http://{}:26300", cli.address);
    let mempool_addr = format!("{}:26000", cli.address);
    let fund_amount = fund_amount_per_sender(&cli);
    let num_tokens = cli.num_markets * 2;

    info!("RPC Address: {}", rpc_addr);
    info!("Mempool Address: {}", mempool_addr);
    info!("Markets: {}", cli.num_markets);
    info!("Tokens: {}", num_tokens);
    info!("Senders: {}", cli.senders);
    info!("Tasks: {}", cli.tasks);
    info!("Rate per task: {} orders/s", cli.rate_per_task);
    info!("Duration: {} seconds", cli.duration);
    info!("Order amount: {}", cli.order_amount);
    info!("Fund amount per sender: {}", fund_amount);

    let creator = Arc::new(Signer::new());
    info!("Creator address: {}", creator.address());

    let client = LightPoolClient::new(&rpc_addr)
        .with_timeout(Duration::from_secs(30));

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

    let baseline_latency = measure_create_token_latency(&client, creator.as_ref()).await?;

    info!("Phase 1: Creating tokens...");
    let _tokens = create_tokens(
        &client,
        creator.as_ref(),
        num_tokens,
        fund_amount,
        cli.senders,
        cli.num_markets,
    )
    .await?;

    info!("Waiting for token creation to be processed...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    info!("Phase 2: Creating markets...");
    let markets = create_markets(
        &client,
        creator.as_ref(),
        &_tokens,
        cli.num_markets,
    )
    .await?;

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
        fund_amount,
    )
    .await?;

    info!("Waiting for sender funding to be processed...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    info!("Measuring place order transaction size...");
    if !senders.is_empty() {
        let sample_sender = &senders[0];
        let sample_market = &markets[sample_sender.market_index];
        match measure_place_order_tx_size(sample_sender, sample_market, cli.order_amount) {
            Ok(size) => {
                info!("Place order transaction size: {} bytes", size);
                info!(
                    "Expected bandwidth per task at max rate: {:.2} KB/s",
                    (size as f64 * cli.rate_per_task as f64) / 1024.0
                );
                info!(
                    "Total expected bandwidth: {:.2} MB/s",
                    (size as f64 * cli.rate_per_task as f64 * cli.tasks as f64) / (1024.0 * 1024.0)
                );
            }
            Err(e) => warn!("Failed to measure transaction size: {}", e),
        }
    }

    info!(
        "Starting burst spot orders: {} tasks over {} senders across {} markets...",
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
        let start_index = task_id * senders_per_task + std::cmp::min(task_id, remaining_senders);
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

        let handle = tokio::spawn(burst_place_order_task(
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

    info!("Sending final RPC place order to measure actual completion time...");
    let final_sender = &senders[0];
    let final_market = &markets[final_sender.market_index];
    let final_tx_success = match (|| async {
        let final_order_params = PlaceOrderParams {
            side: OrderSide::Sell,
            amount: cli.order_amount,
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::GTC,
            },
            limit_price: final_market.ask_price,
            token_address: final_market.base_token,
        };

        let final_place_order_action = ActionBuilder::place_order(
            final_market.market_address,
            final_order_params,
        )
        .map_err(|e| format!("Failed to create final place order action: {}", e))?;

        let final_place_order_tx = TransactionBuilder::new()
            .sender(final_sender.address)
            .expiration(u64::MAX)
            .add_action(final_place_order_action)
            .build_and_sign_only(final_sender.signer.as_ref())
            .map_err(|e| format!("Failed to build final transaction: {}", e))?;

        let final_response = client.submit_transaction(final_place_order_tx).await
            .map_err(|e| format!("Failed to submit final transaction: {}", e))?;

        if !final_response.receipt.is_success() {
            if let ExecutionStatus::Failure(error_msg) = &final_response.receipt.status {
                return Err(format!("Final transaction failed: {}", error_msg));
            }
            return Err("Final transaction failed".to_string());
        }

        Ok(())
    })()
    .await
    {
        Ok(()) => {
            info!("Final RPC place order completed successfully");
            true
        }
        Err(e) => {
            warn!("Final transaction failed (continuing with measurement): {}", e);
            false
        }
    };

    let actual_completion_time = start_time.elapsed();
    let total_orders = counter.load(Ordering::Relaxed);
    let actual_throughput = total_orders as f64 / actual_completion_time.as_secs_f64();

    info!("Multi-market spot burst test completed!");
    info!("========================================");
    info!(
        "Final transaction status: {}",
        if final_tx_success {
            "SUCCESS"
        } else {
            "FAILED (measurement still valid)"
        }
    );
    info!("Markets: {}", cli.num_markets);
    info!("Total orders sent: {}", total_orders);
    info!(
        "Actual completion time: {:.2} seconds",
        actual_completion_time.as_secs_f64()
    );
    info!("Actual tps: {:.1} tx/s", actual_throughput);
    info!("Baseline Latency: {:.3} seconds", baseline_latency.as_secs_f64());

    Ok(())
}
