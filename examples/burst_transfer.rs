// Copyright (c) LightPool Labs
// Author: xiaoyu1998

use lightpool_sdk::{
    LightPoolClient, TransactionBuilder, ActionBuilder, Signer,
    Address, ContractAddress, CreateTokenParams, TransferParams, ObjectID,
    ExecutionStatus,
    extract_token_address_from_events,
    balance_object_id,
};
use lightpool_sdk::lightpool_types::call::{GetBalance, GetBalanceParams};
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

const VALIDATE_SENDERS: usize = 10;

#[derive(Parser)]
#[clap(
    author,
    version,
    about,
    long_about = "Burst client for LightPool token transfers."
)]
struct Cli {
    /// The base address of the node (will generate RPC and mempool addresses from this)
    #[clap(long, default_value = "127.0.0.1")]
    address: String,

    /// Number of sender accounts to fund for parallel burst transfers
    #[clap(long, default_value = "2048")]
    senders: usize,

    /// Number of concurrent burst tasks (e.g. 2, 8, 16)
    #[clap(short, long, default_value = "8")]
    tasks: usize,

    /// Transfer rate per task (transactions per second)
    #[clap(short, long, default_value = "1000")]
    rate_per_task: u64,

    /// Duration to run burst transfers (seconds)
    #[clap(short, long, default_value = "10")]
    duration: u64,

    /// Transfer amount per transaction (smallest units, 6 decimal places)
    #[clap(long, default_value = "2048")]
    transfer_amount: u64,
}

struct BurstSender {
    signer: Arc<Signer>,
    address: Address,
    balance_id: ObjectID,
}

fn fund_amount_per_sender(cli: &Cli) -> u64 {
    cli.transfer_amount
        .saturating_mul(cli.rate_per_task)
        .saturating_mul(cli.duration)
        .saturating_add(cli.transfer_amount)
}

fn burst_recipient_index(sender_index: usize, tx_count: u64, sender_count: usize) -> usize {
    if sender_count <= 1 {
        return 0;
    }
    (sender_index + 1 + tx_count as usize) % sender_count
}

fn burst_transfers_per_account(
    tasks: usize,
    senders: usize,
    total_tx: u64,
) -> (Vec<u64>, Vec<u64>) {
    if senders == 0 || tasks == 0 {
        return (Vec::new(), Vec::new());
    }

    let mut sender_outs = vec![0u64; senders];
    let mut sender_ins = vec![0u64; senders];
    let senders_per_task = senders / tasks;
    let remaining_senders = senders % tasks;
    let base_tx_per_task = total_tx / tasks as u64;
    let extra_task_txs = total_tx % tasks as u64;

    for task_id in 0..tasks {
        let start_index =
            task_id * senders_per_task + std::cmp::min(task_id, remaining_senders);
        let end_index = start_index
            + senders_per_task
            + if task_id < remaining_senders { 1 } else { 0 };
        let range_size = end_index - start_index;
        if range_size == 0 {
            continue;
        }

        let task_txs = base_tx_per_task + if task_id < extra_task_txs as usize { 1 } else { 0 };
        for tx_count in 0..task_txs {
            let sender_index = start_index + (tx_count as usize % range_size);
            let recipient_index = burst_recipient_index(sender_index, tx_count, senders);
            sender_outs[sender_index] += 1;
            sender_ins[recipient_index] += 1;
        }
    }

    (sender_outs, sender_ins)
}

async fn query_token_balance(
    client: &LightPoolClient,
    token: ContractAddress,
    account: Address,
) -> Result<GetBalance, String> {
    let action = ActionBuilder::get_balance(token, account, GetBalanceParams {})
        .map_err(|e| format!("get_balance action: {e}"))?;
    let tx = TransactionBuilder::new()
        .account(account)
        .expiration(u64::MAX)
        .add_action(action)
        .build_and_without_sign()
        .map_err(|e| format!("get_balance tx: {e}"))?;
    let bytes = client
        .call(tx)
        .await
        .map_err(|e| format!("get_balance call: {e}"))?;
    bincode::deserialize(&bytes).map_err(|e| format!("decode GetBalance: {e}"))
}

async fn validate_burst_transfer_balances(
    client: &LightPoolClient,
    token_contract: ContractAddress,
    senders: &[BurstSender],
    total_tx: u64,
    fund_amount: u64,
    transfer_amount: u64,
    tasks: usize,
    final_rpc_transfer_on_first_sender: bool,
) -> Result<(), String> {
    if senders.is_empty() {
        return Ok(());
    }

    let (mut sender_outs, mut sender_ins) =
        burst_transfers_per_account(tasks, senders.len(), total_tx);

    if final_rpc_transfer_on_first_sender {
        sender_outs[0] = sender_outs[0].saturating_add(1);
        let recipient_index = burst_recipient_index(0, 0, senders.len());
        sender_ins[recipient_index] = sender_ins[recipient_index].saturating_add(1);
    }

    let validate_senders = VALIDATE_SENDERS.min(senders.len());
    let mut failures = 0usize;

    info!(
        "Validate transfer balances: first {validate_senders}/{} senders (total_tx={total_tx})...",
        senders.len()
    );

    for (idx, sender) in senders.iter().enumerate().take(validate_senders) {
        let outs = sender_outs[idx];
        let ins = sender_ins[idx];
        let expected_total = fund_amount
            .saturating_sub(outs.saturating_mul(transfer_amount))
            .saturating_add(ins.saturating_mul(transfer_amount));
        let balance = query_token_balance(client, token_contract, sender.address).await?;
        let ok = balance.total == expected_total
            && balance.available == expected_total
            && balance.locked == 0;

        if ok {
            info!(
                "Sender {idx} ({}): PASS outs={outs} ins={ins} total={}",
                sender.address, balance.total
            );
        } else {
            failures += 1;
            error!(
                "Sender {idx} ({}): FAIL outs={outs} ins={ins}",
                sender.address
            );
            error!(
                "  got total={} locked={} avail={}; want total={expected_total} locked=0 avail={expected_total}",
                balance.total, balance.locked, balance.available
            );
        }
    }

    if failures > 0 {
        return Err(format!(
            "{failures} accounts failed burst transfer balance check"
        ));
    }
    Ok(())
}

fn measure_transfer_tx_size(
    sender: &BurstSender,
    token_contract: ContractAddress,
    transfer_amount: u64,
    recipient: Address,
) -> Result<usize, String> {
    let transfer_params = TransferParams {
        to: recipient,
        amount: transfer_amount,
    };

    let transfer_action = ActionBuilder::transfer_token(
        token_contract,
        transfer_params,
    ).map_err(|e| format!("Failed to create transfer action: {}", e))?;

    let transfer_tx = TransactionBuilder::new()
        .sender(sender.address)
        .expiration(u64::MAX)
        .add_action(transfer_action)
        .build_and_sign_only(sender.signer.as_ref())
        .map_err(|e| format!("Failed to build transaction: {}", e))?;

    let tx_bytes = bincode::serialize(&transfer_tx)
        .map_err(|e| format!("Failed to serialize transaction: {}", e))?;

    Ok(tx_bytes.len())
}

async fn create_token(
    client: &LightPoolClient,
    creator: &Signer,
    total_supply: u64,
) -> Result<(ContractAddress, Duration), String> {
    info!("Creating token via RPC...");

    let creator_address = creator.address();

    let create_params = CreateTokenParams {
        name: "BurstTest Token".into(),
        symbol: "BURST".into(),
        total_supply,
        mintable: false,
        to: creator_address,
    };

    let create_action = ActionBuilder::create_token(create_params)
        .map_err(|e| format!("Failed to create token action: {}", e))?;

    let create_tx = TransactionBuilder::new()
        .sender(creator_address)
        .expiration(u64::MAX)
        .add_action(create_action)
        .build_and_sign_only(creator)
        .map_err(|e| format!("Failed to build transaction: {}", e))?;

    let rpc_start = Instant::now();
    let response = client.submit_transaction(create_tx).await
        .map_err(|e| format!("Failed to submit transaction: {}", e))?;
    let rpc_latency = rpc_start.elapsed();

    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(format!("Token creation failed: {}", error_msg));
        }
        return Err("Token creation failed".to_string());
    }

    let token_contract = extract_token_address_from_events(&response.receipt)
        .ok_or_else(|| "Failed to extract token contract from create event".to_string())?;

    info!("Token created successfully:");
    info!("   Token contract: {}", token_contract);

    Ok((token_contract, rpc_latency))
}

async fn fund_burst_senders(
    client: &LightPoolClient,
    creator: &Signer,
    token_contract: ContractAddress,
    sender_count: usize,
    fund_amount: u64,
) -> Result<Vec<BurstSender>, String> {
    if sender_count == 0 {
        return Ok(Vec::new());
    }

    let senders: Vec<BurstSender> = (0..sender_count)
        .map(|_| {
            let signer = Arc::new(Signer::new());
            let address = signer.address();
            let balance_id = balance_object_id(token_contract, address);
            BurstSender {
                signer,
                address,
                balance_id,
            }
        })
        .collect();

    let creator_address = creator.address();

    info!(
        "Funding {} sender accounts with {} each in one transaction...",
        senders.len(),
        fund_amount,
    );

    let mut tx_builder = TransactionBuilder::new()
        .sender(creator_address)
        .expiration(u64::MAX);

    for sender in &senders {
        let transfer_params = TransferParams {
            to: sender.address,
            amount: fund_amount,
        };
        let transfer_action = ActionBuilder::transfer_token(
            token_contract,
            transfer_params,
        ).map_err(|e| format!("Failed to create fund transfer action: {}", e))?;
        tx_builder = tx_builder.add_action(transfer_action);
    }

    let fund_tx = tx_builder
        .build_and_sign_only(creator)
        .map_err(|e| format!("Failed to build fund transaction: {}", e))?;

    let response = client.submit_transaction(fund_tx).await
        .map_err(|e| format!("Failed to submit fund transaction: {}", e))?;

    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(format!("Fund transaction failed: {}", error_msg));
        }
        return Err("Fund transaction failed".to_string());
    }

    info!("Funded {} sender accounts", senders.len());
    Ok(senders)
}

async fn fund_burst_senders_with_retry(
    client: &LightPoolClient,
    creator: &Signer,
    token_contract: ContractAddress,
    sender_count: usize,
    fund_amount: u64,
) -> Result<Vec<BurstSender>, String> {
    const MAX_ATTEMPTS: u32 = 120;

    for attempt in 1..=MAX_ATTEMPTS {
        match fund_burst_senders(
            client,
            creator,
            token_contract,
            sender_count,
            fund_amount,
        )
        .await
        {
            Ok(senders) => return Ok(senders),
            Err(error)
                if error.contains("Balance object not found") && attempt < MAX_ATTEMPTS =>
            {
                warn!(
                    "Fund attempt {}/{} failed (balance not ready): {}",
                    attempt, MAX_ATTEMPTS, error
                );
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(error) => return Err(error),
        }
    }

    Err("Fund transaction failed after waiting for creator balance".to_string())
}

async fn burst_transfer_task(
    task_id: usize,
    mempool_addr: String,
    senders: Arc<Vec<BurstSender>>,
    start_index: usize,
    end_index: usize,
    token_contract: ContractAddress,
    rate_per_second: u64,
    duration_secs: u64,
    transfer_amount: u64,
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
        let recipient_index = burst_recipient_index(sender_index, tx_count, senders.len());
        let recipient = senders[recipient_index].address;

        let transfer_params = TransferParams {
            to: recipient,
            amount: transfer_amount,
        };

        let transfer_action = ActionBuilder::transfer_token(
            token_contract,
            transfer_params,
        ).map_err(|e| format!("Task {}: Failed to create transfer action: {}", task_id, e))?;

        let transfer_tx = TransactionBuilder::new()
            .sender(sender.address)
            .expiration(expiration)
            .add_action(transfer_action)
            //.build_and_sign_only(sender.signer.as_ref())
            .build_and_without_sign()
            .map_err(|e| format!("Task {}: Failed to build transaction: {}", task_id, e))?;

        let tx_bytes = bincode::serialize(&transfer_tx)
            .map_err(|e| format!("Task {}: Failed to serialize transaction: {}", task_id, e))?;

        if let Err(e) = transport.send(Bytes::from(tx_bytes)).await {
            warn!("Task {}: Failed to send transaction: {}", task_id, e);
            break;
        }

        tx_count += 1;
        expiration = expiration.saturating_sub(1);
        counter.fetch_add(1, Ordering::Relaxed);
    }

    info!(
        "Task {} completed (sender range {}-{}). Sent {} transactions",
        task_id, start_index, end_index, tx_count
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    if cli.senders == 0 {
        return Err("--senders must be at least 1".to_string());
    }
    if cli.senders == 1 {
        return Err("--senders must be at least 2 for peer burst transfers".to_string());
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

    info!("LightPool Burst Client");
    info!("========================");
    info!("Address: {}", cli.address);

    let rpc_addr = format!("http://{}:26300", cli.address);
    let mempool_addr = format!("{}:26000", cli.address);
    let fund_amount = fund_amount_per_sender(&cli);
    let total_supply = fund_amount
        .checked_mul(cli.senders as u64)
        .ok_or("Total supply overflow")?;

    info!("RPC Address: {}", rpc_addr);
    info!("Mempool Address: {}", mempool_addr);
    info!("Senders: {}", cli.senders);
    info!("Tasks: {}", cli.tasks);
    info!("Rate per task: {} tx/s", cli.rate_per_task);
    info!("Duration: {} seconds", cli.duration);
    info!("Transfer amount: {}", cli.transfer_amount);
    info!("Fund amount per sender: {}", fund_amount);
    info!("Create total supply: {}", total_supply);

    let creator = Arc::new(Signer::new());
    let creator_address = creator.address();
    info!("Creator address: {}", creator_address);

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

    info!("Measuring baseline processing latency with token creation...");
    let (token_contract, baseline_latency) =
        create_token(&client, creator.as_ref(), total_supply).await?;

    info!("Waiting for token creation to be processed...");
    let senders = fund_burst_senders_with_retry(
        &client,
        creator.as_ref(),
        token_contract,
        cli.senders,
        fund_amount,
    ).await?;

    info!("Waiting for sender funding to be processed...");
    tokio::time::sleep(Duration::from_secs(1)).await;

    info!("Measuring transfer transaction size...");
    if senders.len() >= 2 {
        let sample_sender = &senders[0];
        let sample_recipient = senders[burst_recipient_index(0, 0, senders.len())].address;
        match measure_transfer_tx_size(
            sample_sender,
            token_contract,
            cli.transfer_amount,
            sample_recipient,
        ) {
            Ok(size) => {
                info!("Transfer transaction size: {} bytes", size);
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
        "Starting burst transfers: {} tasks over {} senders...",
        cli.tasks, senders.len()
    );

    let senders = Arc::new(senders);
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

        let handle = tokio::spawn(burst_transfer_task(
            task_id,
            mempool_addr.clone(),
            Arc::clone(&senders),
            start_index,
            end_index,
            token_contract,
            cli.rate_per_task,
            cli.duration,
            cli.transfer_amount,
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
                "Progress [{:2}/{}]: {} total tx, {:.1} tx/s",
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
    let total_tx = counter.load(Ordering::Relaxed);
    let mempool_send_rate = total_tx as f64 / mempool_send_time.as_secs_f64();

    info!("Mempool phase completed:");
    info!("   Total transactions sent to mempool: {}", total_tx);
    info!("   Mempool send time: {:.2} seconds", mempool_send_time.as_secs_f64());
    info!("   Mempool send rate: {:.1} tx/s", mempool_send_rate);

    info!("Sending final RPC transaction to measure actual completion time...");
    let final_sender = &senders[0];
    let final_recipient = senders[burst_recipient_index(0, 0, senders.len())].address;
    let final_tx_success = match (|| async {
        let final_transfer_params = TransferParams {
            to: final_recipient,
            amount: cli.transfer_amount,
        };

        let final_transfer_action = ActionBuilder::transfer_token(
            token_contract,
            final_transfer_params,
        ).map_err(|e| format!("Failed to create final transfer action: {}", e))?;

        let final_transfer_tx = TransactionBuilder::new()
            .sender(final_sender.address)
            .expiration(u64::MAX)
            .add_action(final_transfer_action)
            .build_and_sign_only(final_sender.signer.as_ref())
            .map_err(|e| format!("Failed to build final transaction: {}", e))?;

        let final_response = client.submit_transaction(final_transfer_tx).await
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
            info!("Final RPC transaction completed successfully");
            true
        }
        Err(e) => {
            warn!("Final transaction failed (continuing with measurement): {}", e);
            false
        }
    };

    let actual_completion_time = start_time.elapsed();
    let total_tx = counter.load(Ordering::Relaxed);
    let actual_throughput = total_tx as f64 / actual_completion_time.as_secs_f64();

    info!("Burst test completed!");
    info!("========================");
    info!(
        "Final transaction status: {}",
        if final_tx_success { "SUCCESS" } else { "FAILED (measurement still valid)" }
    );
    info!("Total transactions sent: {}", total_tx);
    info!("Actual completion time: {:.2} seconds", actual_completion_time.as_secs_f64());
    info!("Actual tps: {:.1} tx/s", actual_throughput);
    info!("Baseline Latency: {:.3} seconds", baseline_latency.as_secs_f64());

    info!(
        "Validating transfer balances for first {} senders via get_balance...",
        VALIDATE_SENDERS
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
    validate_burst_transfer_balances(
        &client,
        token_contract,
        senders.as_ref(),
        total_tx,
        fund_amount,
        cli.transfer_amount,
        cli.tasks,
        final_tx_success,
    )
    .await?;

    Ok(())
}
