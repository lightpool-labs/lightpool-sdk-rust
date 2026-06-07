use lightpool_sdk::{
    LightPoolClient, TransactionBuilder, ActionBuilder, Signer,
    Address, CreateTokenParams, CreateMarketParams, PlaceOrderParams, TransferParams, ObjectID,
    TransactionReceipt, EventType, EventData, ExecutionStatus,
    extract_token_id_from_events, extract_token_address_from_events,
    extract_balance_id_from_events, extract_market_id_from_events,
    extract_market_address_from_events, print_receipt_json, print_spot_receipt_json,
    OrderSide, TimeInForce, OrderParamsType, MarketState, SideBookSize,
    TokenCreatedEvent, MarketCreatedEvent,
};
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tokio::sync::Semaphore;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::sink::SinkExt;
use bytes::Bytes;
use clap::Parser;
use env_logger::Env;
use log::{info, warn, error, debug};
use compact_str::CompactString;
use rand::Rng;
use lightpool_sdk::lightpool_types::address_type::{TOKEN_CONTRACT_ADDRESS, SPOT_CONTRACT_ADDRESS};

/// Market filling phases
#[derive(Debug, Clone, Copy, PartialEq)]
enum MarketPhase {
    /// Filling market depth (placing non-matching orders)
    FillingDepth,
    /// Matching phase - going up with bids
    MatchingUpBids,
    /// Matching phase - going down with asks
    MatchingDownAsks,
}

#[derive(Parser)]
#[clap(
    author,
    version,
    about,
    long_about = "Burst client for LightPool spot trading transactions."
)]
struct SpotBenchmarkCli {
    /// The base address of the node (will generate RPC and mempool addresses from this)
    #[clap(long, default_value = "127.0.0.1")]
    address: String,

    /// Number of markets to create
    #[clap(long, default_value = "500")]
    num_markets: usize,

    /// Number of concurrent tasks for burst trading
    #[clap(short, long, default_value = "50")]
    tasks: usize,

    /// Trading rate per task (orders per second)
    #[clap(short, long, default_value = "10000")]
    rate_per_task: u64,

    /// Duration to run burst trading (seconds)
    #[clap(short, long, default_value = "10")]
    duration: u64,

    /// Order amount per transaction (in smallest units)
    #[clap(long, default_value = "1000000")]
    order_amount: u64,

    /// Price range for orders (in smallest units)
    #[clap(long, default_value = "10000000")]
    price_range: u64,
}

/// Market with associated token balances and price control
#[derive(Debug, Clone)]
struct MarketWithBalances {
    market_id: ObjectID,
    market_address: Address,
    base_token: Address,        // Base token address (e.g., BTC)
    quote_token: Address,       // Quote token address (e.g., USDT)
    sender_base_balance_id: ObjectID,  // Sender's base token balance ID
    sender_quote_balance_id: ObjectID, // Sender's quote token balance ID
    base_price: u64,           // Base price for this market
    tick_size: u64,            // Minimum price increment
    bid_levels_used: u32,      // Number of bid price levels used
    ask_levels_used: u32,      // Number of ask price levels used
    max_levels: u32,           // Maximum levels per side (50)
    min_bid_price: u64,        // Lowest bid price used
    max_ask_price: u64,        // Highest ask price used
    current_price: u64,        // Current price for directional movement
    direction: bool,           // true = up (bids), false = down (asks)
    phase: MarketPhase,        // Current phase of market filling
}

impl MarketWithBalances {
    fn new(
        market_id: ObjectID, 
        market_address: Address,
        base_token: Address,
        quote_token: Address,
        sender_base_balance_id: ObjectID,
        sender_quote_balance_id: ObjectID,
        base_price: u64, 
        tick_size: u64
    ) -> Self {
        let max_levels = 20;
        Self {
            market_id,
            market_address,
            base_token,
            quote_token,
            sender_base_balance_id,
            sender_quote_balance_id,
            base_price,
            tick_size,
            bid_levels_used: 0,
            ask_levels_used: 0,
            max_levels,
            min_bid_price: base_price - (max_levels as u64 * tick_size),  // Start bids below base price
            max_ask_price: base_price + (max_levels as u64 * tick_size),  // Start asks above base price
            current_price: base_price,        // Start current price at base price
            direction: true,                  // Start direction as up
            phase: MarketPhase::FillingDepth, // Start with filling depth
        }
    }

    /// Get next bid price level (from smaller to bigger price)
    fn get_next_bid_price(&mut self) -> Option<u64> {
        if self.bid_levels_used >= self.max_levels {
            return None;
        }
        
        let price = self.min_bid_price + (self.bid_levels_used as u64 * self.tick_size);
        self.bid_levels_used += 1;
        Some(price)
    }

    /// Get next ask price level (from bigger to smaller price)
    fn get_next_ask_price(&mut self) -> Option<u64> {
        if self.ask_levels_used >= self.max_levels {
            return None;
        }
        
        let price = self.max_ask_price - (self.ask_levels_used as u64 * self.tick_size);
        if price > 0 {
            self.ask_levels_used += 1;
            Some(price)
        } else {
            None
        }
    }


    /// Check if both sides are full
    fn is_full(&self) -> bool {
        self.bid_levels_used >= self.max_levels && self.ask_levels_used >= self.max_levels
    }

    /// Get current depth
    fn get_depth(&self) -> (u32, u32) {
        (self.bid_levels_used, self.ask_levels_used)
    }

    /// Get next matching bid price (going up)
    fn get_next_matching_bid_price(&mut self) -> Option<u64> {
        let price = self.current_price + self.tick_size;
        
        // Check if we've reached the top (max ask price)
        if price >= self.max_ask_price {
            // Switch to going down with asks
            self.phase = MarketPhase::MatchingDownAsks;
            self.current_price = self.max_ask_price + self.tick_size; // Start above max_ask_price
            return None;
        }
        
        self.current_price = price;
        Some(price)
    }

    /// Get next matching ask price (going down)
    fn get_next_matching_ask_price(&mut self) -> Option<u64> {
        let price = self.current_price - self.tick_size;
        
        // Check if we've reached the bottom (min bid price)
        if price <= self.min_bid_price {
            // Switch to going up with bids
            self.phase = MarketPhase::MatchingUpBids;
            self.current_price = self.min_bid_price - self.tick_size; // Start below min_bid_price
            return None;
        }
        
        self.current_price = price;
        Some(price)
    }

    /// Get next order based on current phase
    fn get_next_order(&mut self) -> Option<(OrderSide, u64)> {
        match self.phase {
            MarketPhase::FillingDepth => {
                // Fill market depth systematically
                if self.bid_levels_used < self.max_levels {
                    // Fill bid side first
                    if let Some(price) = self.get_next_bid_price() {
                        return Some((OrderSide::Buy, price));
                    }
                } else if self.ask_levels_used < self.max_levels {
                    // Fill ask side
                    if let Some(price) = self.get_next_ask_price() {
                        return Some((OrderSide::Sell, price));
                    }
                } else {
                    // Both sides are full, switch to matching phase
                    self.phase = MarketPhase::MatchingUpBids;
                    self.current_price = self.min_bid_price - self.tick_size; // Start below min_bid_price
                    return self.get_next_order(); // Recursive call for matching phase
                }
                None
            }
            MarketPhase::MatchingUpBids => {
                // Going up with bids to match asks
                if let Some(price) = self.get_next_matching_bid_price() {
                    Some((OrderSide::Buy, price))
                } else {
                    // Switched to going down, get next order
                    self.get_next_order()
                }
            }
            MarketPhase::MatchingDownAsks => {
                // Going down with asks to match bids
                if let Some(price) = self.get_next_matching_ask_price() {
                    Some((OrderSide::Sell, price))
                } else {
                    // Switched to going up, get next order
                    self.get_next_order()
                }
            }
        }
    }
}



/// Measure the size of a place order transaction in bytes
fn measure_place_order_tx_size(
    signer: &Signer,
    market_address: Address,
    market_id: ObjectID,
    balance_id: ObjectID,
    order_amount: u64,
    price: u64,
) -> Result<usize, String> {
    let sender_address = signer.address();
    
    let order_params = PlaceOrderParams {
        side: OrderSide::Buy,
        amount: order_amount,
        order_type: OrderParamsType::Limit {
            tif: TimeInForce::GTC,
        },
        limit_price: price,
    };
    
    let place_order_action = ActionBuilder::place_order(
        market_address,
        market_id,
        balance_id,
        order_params,
    ).map_err(|e| format!("Failed to create place order action: {}", e))?;
    
    let place_order_tx = TransactionBuilder::new()
        .sender(sender_address)
        .expiration(u64::MAX)
        .add_action(place_order_action)
        .build_and_sign_only(&signer)
        .map_err(|e| format!("Failed to build transaction: {}", e))?;
    
    let tx_bytes = bincode::serialize(&place_order_tx)
        .map_err(|e| format!("Failed to serialize transaction: {}", e))?;
    
    Ok(tx_bytes.len())
}

/// Create tokens in batch using single transaction
async fn create_tokens_batch(
    client: &LightPoolClient,
    signers: Vec<Arc<Signer>>,
    num_tokens: usize,
) -> Result<Vec<(Address, ObjectID, ObjectID)>, String> {
    info!("Creating {} tokens in a single transaction...", num_tokens);
    
    // Use the first signer for the batch transaction
    let signer = &signers[0];
    let sender_address = signer.address();
    
    // Create all token creation actions at once
    let mut create_actions = Vec::with_capacity(num_tokens);
    for i in 0..num_tokens {
        let create_params = CreateTokenParams {
            name: format!("Token{}", i + 1).into(),
            symbol: format!("TKN{}", i + 1).into(),
            total_supply: 1_000_000_000_000_000, // 1 billion tokens with 6 decimals
            mintable: true,
            to: sender_address,
        };
        
        let create_action = ActionBuilder::create_token(
            TOKEN_CONTRACT_ADDRESS,
            create_params,
        ).map_err(|e| format!("Failed to create token action: {}", e))?;
        
        create_actions.push(create_action);
    }
    
    info!("Created {} token creation actions", create_actions.len());
    
    // Build transaction with all token creation actions
    let mut tx_builder = TransactionBuilder::new()
        .sender(sender_address)
        .expiration(u64::MAX);
    
    for action in create_actions {
        tx_builder = tx_builder.add_action(action);
    }
    
    let create_tx = tx_builder
        .build_and_sign_only(&signer)
        .map_err(|e| format!("Failed to build token creation transaction: {}", e))?;
    
    // Try to serialize the transaction to verify it's valid
    match bincode::serialize(&create_tx) {
        Ok(bytes) => info!("Token creation transaction serialized successfully, size: {} bytes", bytes.len()),
        Err(e) => return Err(format!("Failed to serialize token creation transaction: {}", e)),
    }
    
    // Submit the transaction with all token creations
    info!("Submitting transaction with {} token creation actions...", num_tokens);
    let response = client.submit_transaction(create_tx).await
        .map_err(|e| format!("Failed to submit token creation transaction: {}", e))?;
    
    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(format!("Token creation failed: {}", error_msg));
        }
        return Err("Token creation failed".to_string());
    }
    
    // Extract token information from the transaction events
    let mut tokens = Vec::new();
    let mut token_count = 0;
    
    for event in &response.receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "token_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(token_created_event) = bincode::deserialize::<TokenCreatedEvent>(data) {
                        tokens.push((
                            token_created_event.token_address,
                            token_created_event.token_id,
                            token_created_event.balance_id
                        ));
                        token_count += 1;
                    }
                }
            }
        }
    }
    
    if tokens.len() != num_tokens {
        return Err(format!("Expected {} tokens from creation events, got {}", num_tokens, tokens.len()));
    }
    
    info!("Successfully created {} tokens in a single transaction", tokens.len());
    Ok(tokens)
}

/// Create markets in batch using single transaction
async fn create_markets_batch(
    client: &LightPoolClient,
    signers: Vec<Arc<Signer>>,
    tokens: Vec<(Address, ObjectID, ObjectID)>,
    num_markets: usize,
) -> Result<Vec<MarketWithBalances>, String> {
    info!("Creating {} markets in a single transaction...", num_markets);
    
    // Use the first signer for the batch transaction
    let signer = &signers[0];
    let sender_address = signer.address();
    
    // Create all market creation actions at once
    let mut market_actions = Vec::with_capacity(num_markets);
    
    for i in 0..num_markets {
        // Select two different tokens for this market
        let token1_index = i * 2;
        let token2_index = i * 2 + 1;
        
        if token2_index >= tokens.len() {
            return Err(format!("Not enough tokens for market {} (need 2, have {})", i, tokens.len()));
        }
        
        let (base_token_address, base_token_id, _) = tokens[token1_index];
        let (quote_token_address, quote_token_id, _) = tokens[token2_index];
        
        // Generate realistic market parameters
        let base_price = 10_000_000 + (i as u64 * 1_000_000); // 10-110 USDT base price
        let tick_size = 100_000; // 0.1 USDT tick size
        let min_order_size = 100_000; // 0.1 base token minimum
        
        let market_create_params = CreateMarketParams {
            name: format!("Market{}/Market{}", token1_index + 1, token2_index + 1).into(),
            base_token: base_token_address,
            quote_token: quote_token_address,
            min_order_size,
            tick_size,
            maker_fee_bps: 10,        // 0.1% maker fee
            taker_fee_bps: 20,        // 0.2% taker fee
            allow_market_orders: true,
            state: MarketState::Active,
            limit_order: true,
            side_book_size: SideBookSize::Middle,
        };
        
        let market_create_action = ActionBuilder::create_market(
            SPOT_CONTRACT_ADDRESS,
            market_create_params,
        ).map_err(|e| format!("Failed to create market action: {}", e))?;
        
        market_actions.push(market_create_action);
    }
    
    info!("Created {} market creation actions", market_actions.len());
    
    // Build transaction with all market creation actions
    let mut tx_builder = TransactionBuilder::new()
        .sender(sender_address)
        .expiration(u64::MAX);
    
    for action in market_actions {
        tx_builder = tx_builder.add_action(action);
    }
    
    let market_create_tx = tx_builder
        .build_and_sign_only(&signer)
        .map_err(|e| format!("Failed to build market creation transaction: {}", e))?;
    
    // Try to serialize the transaction to verify it's valid
    match bincode::serialize(&market_create_tx) {
        Ok(bytes) => info!("Market creation transaction serialized successfully, size: {} bytes", bytes.len()),
        Err(e) => return Err(format!("Failed to serialize market creation transaction: {}", e)),
    }
    
    // Submit the transaction with all market creations
    info!("Submitting transaction with {} market creation actions...", num_markets);
    let response = client.submit_transaction(market_create_tx).await
        .map_err(|e| format!("Failed to submit market creation transaction: {}", e))?;
    
    if !response.receipt.is_success() {
        if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
            return Err(format!("Market creation failed: {}", error_msg));
        }
        return Err("Market creation failed".to_string());
    }
    
    // Extract market information from the transaction events
    let mut markets = Vec::new();
    let mut market_count = 0;
    
    for event in &response.receipt.events {
        if let EventType::Call(action_name) = &event.event_type {
            if action_name == "market_created" {
                if let EventData::Bytes(data) = &event.data {
                    if let Ok(market_created_event) = bincode::deserialize::<MarketCreatedEvent>(data) {
                        // Get the corresponding token pair for this market
                        let market_index = market_count;
                        let token1_index = market_index * 2;
                        let token2_index = market_index * 2 + 1;
                        
                        if token2_index < tokens.len() {
                            let (base_token_address, _, base_balance_id) = tokens[token1_index];
                            let (quote_token_address, _, quote_balance_id) = tokens[token2_index];
                            
                            let base_price = 10_000_000 + (market_index as u64 * 1_000_000);
                            let tick_size = 100_000;
                            
                            let market = MarketWithBalances::new(
                                market_created_event.market_id,
                                market_created_event.market_address,
                                base_token_address,
                                quote_token_address,
                                base_balance_id,  // Sender's base token balance ID
                                quote_balance_id,  // Sender's quote token balance ID
                                base_price,
                                tick_size,
                            );
                            
                            markets.push(market);
                            market_count += 1;
                        }
                    }
                }
            }
        }
    }
    
    if markets.len() != num_markets {
        return Err(format!("Expected {} markets from creation events, got {}", num_markets, markets.len()));
    }
    
    info!("Successfully created {} markets in a single transaction", markets.len());
    Ok(markets)
}



/// High-frequency burst place order task with proper market-balance matching
async fn burst_place_order_task(
    task_id: usize,
    mempool_addr: String,
    signer: Arc<Signer>,
    mut markets: Vec<MarketWithBalances>,
    rate_per_second: u64,
    duration_secs: u64,
    order_amount: u64,
    counter: Arc<AtomicU64>,
    semaphore: Arc<Semaphore>,
) -> Result<(), String> {
    let _permit = semaphore.acquire().await.map_err(|e| format!("Failed to acquire semaphore: {}", e))?;
    
    debug!("Task {} starting HF place orders at {} orders/s", task_id, rate_per_second);
    
    // Connect to mempool
    let stream = TcpStream::connect(&mempool_addr).await
        .map_err(|e| format!("Task {}: Failed to connect to mempool: {}", task_id, e))?;
    
    let mut transport = Framed::new(stream, LengthDelimitedCodec::new());
    
    let sender_address = signer.address();
    
    // Calculate interval for target rate
    let interval_micros = if rate_per_second == 0 {
        1000
    } else {
        (1_000_000.0 / rate_per_second as f64) as u64
    };
    let burst_interval = Duration::from_micros(interval_micros.max(1));
    
    let end_time = Instant::now() + Duration::from_secs(duration_secs);
    let mut order_count = 0u64;
    let mut expiration = u64::MAX;
    let mut rng = rand::thread_rng();
    
    debug!("Task {} starting burst with interval: {:?}", task_id, burst_interval);
    
    while Instant::now() < end_time {
        let start_burst = Instant::now();
        
        // Get market and determine order side/price
        let (market_index, side, price) = {
            if markets.is_empty() {
                // No markets available, wait a bit and continue
                tokio::time::sleep(Duration::from_millis(1)).await;
                continue;
            }
            
            let market_index = (order_count as usize) % markets.len();
            let market = &mut markets[market_index];
            
            // Use the new systematic approach
            if let Some((order_side, order_price)) = market.get_next_order() {
                (market_index, order_side, order_price)
            } else {
                // No more orders for this market, wait a bit and continue
                tokio::time::sleep(Duration::from_millis(1)).await;
                continue;
            }
        };
        
        // Get market details and select appropriate balance
        let (market_address, market_id, balance_id) = {
            let market = &markets[market_index];
            
            // Select sender's balance ID based on order side
            let balance_id = match side {
                OrderSide::Buy => market.sender_quote_balance_id,  // Need sender's quote token balance to buy base token
                OrderSide::Sell => market.sender_base_balance_id,  // Need sender's base token balance to sell
            };
            
            (market.market_address, market.market_id, balance_id)
        };
        
        // Create order parameters with controlled price and size
        let order_params = PlaceOrderParams {
            side,
            amount: order_amount,  // Fixed order size
            order_type: OrderParamsType::Limit {
                tif: TimeInForce::GTC,
            },
            limit_price: price,    // Controlled price level
        };
        
        // Build place order action
        let place_order_action = ActionBuilder::place_order(
            market_address,
            market_id,
            balance_id,
            order_params,
        ).map_err(|e| format!("Task {}: Failed to create place order action: {}", task_id, e))?;
        
        // Build transaction
        let place_order_tx = TransactionBuilder::new()
            .sender(sender_address)
            .expiration(expiration)
            .add_action(place_order_action)
            .build_and_verify(&signer)
            .map_err(|e| format!("Task {}: Failed to build transaction: {}", task_id, e))?;
        
        // Serialize and send transaction
        let tx_bytes = bincode::serialize(&place_order_tx)
            .map_err(|e| format!("Task {}: Failed to serialize transaction: {}", task_id, e))?;
        
        if let Err(e) = transport.send(Bytes::from(tx_bytes)).await {
            warn!("Task {}: Failed to send place order transaction: {}", task_id, e);
            break;
        }
        
        order_count += 1;
        expiration = expiration.saturating_sub(1);
        counter.fetch_add(1, Ordering::Relaxed);
        
        // Ultra-fast rate limiting
        let elapsed = start_burst.elapsed();
        if elapsed < burst_interval {
            if burst_interval.as_micros() < 100 {
                while Instant::now().duration_since(start_burst) < burst_interval {
                    std::hint::spin_loop();
                }
            } else {
                tokio::time::sleep(burst_interval - elapsed).await;
            }
        }
    }
    
    info!("Task {} completed. Sent {} place order transactions", task_id, order_count);
    Ok(())
}

/// Monitor order book depth during burst
async fn monitor_order_book_depth(
    markets: Arc<Mutex<Vec<MarketWithBalances>>>,
    duration_secs: u64,
) {
    let start_time = Instant::now();
    let end_time = start_time + Duration::from_secs(duration_secs);
    
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    
    while Instant::now() < end_time {
        interval.tick().await;
        
        let markets_guard = markets.lock().unwrap();
        let total_bids: u32 = markets_guard.iter().map(|m| m.bid_levels_used).sum();
        let total_asks: u32 = markets_guard.iter().map(|m| m.ask_levels_used).sum();
        
        info!("Order Book Depth - Total Bid Levels: {}, Total Ask Levels: {}, Markets: {}", 
              total_bids, total_asks, markets_guard.len());
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = SpotBenchmarkCli::parse();
    
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();
    
    info!("🚀 LightPool Spot Trading Burst Client");
    info!("======================================");
    info!("Address: {}", cli.address);
    
    // Generate RPC and mempool addresses from base address
    let rpc_addr = format!("http://{}:26300", cli.address);
    let mempool_addr = format!("{}:26000", cli.address);
    
    info!("RPC Address: {}", rpc_addr);
    info!("Mempool Address: {}", mempool_addr);
    info!("Markets to create: {}", cli.num_markets);
    info!("Tokens to create: {} (2 per market)", cli.num_markets * 2);
    info!("Tasks: {}", cli.tasks);
    info!("Rate per task: {} orders/s", cli.rate_per_task);
    info!("Duration: {} seconds", cli.duration);
    info!("Order amount: {}", cli.order_amount);
    
    // Create signer for the trader (single trader since we use batch transactions)
    let signer = Arc::new(Signer::new());
    let sender_address = signer.address();
    let signers = vec![signer.clone()];
    info!("Trader address: {}", signer.address());
    
    // Create RPC client
    let client = LightPoolClient::new(&rpc_addr)
        .with_timeout(Duration::from_secs(30));
    
    // Test RPC connectivity
    info!("Testing RPC connection...");
    match client.health_check().await {
        Ok(true) => info!("✅ RPC node is healthy"),
        Ok(false) => {
            error!("⚠️ RPC node responded but not healthy");
            return Ok(());
        }
        Err(e) => {
            error!("❌ Failed to connect to RPC node: {}", e);
            return Ok(());
        }
    }
    
    // Phase 1: Create tokens
    let num_tokens = cli.num_markets * 2;
    info!("Phase 1: Creating {} tokens...", num_tokens);
    let tokens = create_tokens_batch(&client, signers.clone(), num_tokens).await?;
    
    // Wait for token creation to be processed
    info!("Waiting for token creation to be processed...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Phase 2: Create markets
    info!("Phase 2: Creating {} markets...", cli.num_markets);
    let markets = create_markets_batch(&client, signers.clone(), tokens.clone(), cli.num_markets).await?;
    let markets = Arc::new(Mutex::new(markets));
    
    // Print the first market address
    if let Some(first_market) = markets.lock().unwrap().first() {
        info!("First market address: {}", first_market.market_address);
    }
    
    // Wait for market creation to be processed
    info!("Waiting for market creation to be processed...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Measure order transaction size
    info!("📏 Measuring place order transaction size...");
    if let Some(first_market) = markets.lock().unwrap().first() {
        match measure_place_order_tx_size(&signers[0], first_market.market_address, first_market.market_id, first_market.sender_quote_balance_id, cli.order_amount, first_market.base_price) {
            Ok(size) => {
                info!("✅ Place order transaction size: {} bytes", size);
                info!("   Expected bandwidth per task at max rate: {:.2} KB/s", 
                      (size as f64 * cli.rate_per_task as f64) / 1024.0);
                info!("   Total expected bandwidth: {:.2} MB/s", 
                      (size as f64 * cli.rate_per_task as f64 * cli.tasks as f64) / (1024.0 * 1024.0));
            },
            Err(e) => warn!("Failed to measure transaction size: {}", e),
        }
    }
    
    info!("🔥 Starting burst spot trading...");
    
    // Semaphore to limit concurrent connections
    let semaphore = Arc::new(Semaphore::new(cli.tasks));
    
    // Counter for total transactions sent
    let counter = Arc::new(AtomicU64::new(0));
    
    let start_time = Instant::now();
    
    // Split markets for each task
    let mut handles = Vec::new();
    let all_markets = markets.lock().unwrap().clone();
    let market_count = all_markets.len();
    let markets_per_task = market_count / cli.tasks;
    let remaining_markets = market_count % cli.tasks;
    
    info!("Distributing {} markets across {} tasks: {} per task, {} remainder", 
          market_count, cli.tasks, markets_per_task, remaining_markets);
    
    for task_id in 0..cli.tasks {
        // Calculate start and end market indices for this task
        let start_market = task_id * markets_per_task + std::cmp::min(task_id, remaining_markets);
        let end_market = start_market + markets_per_task + if task_id < remaining_markets { 1 } else { 0 };
        
        // Split markets for this task
        let task_markets = all_markets[start_market..end_market].to_vec();
        
        info!("Task {}: assigned {} markets (range {}-{})", 
              task_id, task_markets.len(), start_market, end_market);
        
        let handle = tokio::spawn(burst_place_order_task(
            task_id,
            mempool_addr.clone(),
            signer.clone(), // Use single signer for all tasks
            task_markets,
            cli.rate_per_task,
            cli.duration,
            cli.order_amount,
            counter.clone(),
            semaphore.clone(),
        ));
        handles.push(handle);
    }
    
    // Monitor progress
    let monitor_counter = counter.clone();
    let monitor_handle = tokio::spawn(async move {
        let mut last_count = 0u64;
        let mut last_time = Instant::now();
        
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        for i in 0..cli.duration {
            interval.tick().await;
            
            let current_count = monitor_counter.load(Ordering::Relaxed);
            let current_time = Instant::now();
            let duration = current_time.duration_since(last_time).as_secs_f64();
            let rate = (current_count - last_count) as f64 / duration;
            
            info!("Progress [{:2}/{}]: {} total orders, {:.1} orders/s", 
                  i + 1, cli.duration, current_count, rate);
            
            last_count = current_count;
            last_time = current_time;
        }
    });
    
    // Wait for all tasks to complete
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(())) => {},
            Ok(Err(e)) => error!("Task {} failed: {}", i, e),
            Err(e) => error!("Task {} panicked: {}", i, e),
        }
    }
    
    monitor_handle.abort();
    
    // Calculate mempool send metrics (before final transaction)
    let mempool_send_time = start_time.elapsed();
    let total_orders = counter.load(Ordering::Relaxed);
    let mempool_send_rate = total_orders as f64 / mempool_send_time.as_secs_f64();
    
    info!("Mempool phase completed:");
    info!("   Total orders sent to mempool: {}", total_orders);
    info!("   Mempool send time: {:.2} seconds", mempool_send_time.as_secs_f64());
    info!("   Mempool send rate: {:.1} orders/s", mempool_send_rate);
    
    // Send a final transfer transaction via RPC to measure actual completion time
    info!("Sending final RPC transfer transaction to measure actual completion time...");
    let final_tx_start = Instant::now();
    
    // Use the first token balance which is statistically less likely to have been selected during burst
    let final_balance_id = tokens[0].2; // Get balance_id from first token
    let final_token_address = tokens[0].0; // Get token_address from first token
    
    // Use a smaller amount to avoid insufficient balance issues after multiple orders
    let final_transfer_amount = std::cmp::min(cli.order_amount, 1);
    
    // Attempt final transaction but don't let failure prevent measurement
    let final_tx_success = match (|| async {
        let final_transfer_params = TransferParams {
            to: sender_address, // Transfer to self for testing
            amount: final_transfer_amount,
        };
        
        let final_transfer_action = ActionBuilder::transfer_token(
            final_token_address,
            final_balance_id,
            final_transfer_params,
        ).map_err(|e| format!("Failed to create final transfer action: {}", e))?;
        
        let final_transfer_tx = TransactionBuilder::new()
            .sender(sender_address)
            .expiration(u64::MAX) // Use max expiration for final test
            .add_action(final_transfer_action)
            .build_and_sign_only(&signer)
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
    })().await {
        Ok(()) => {
            info!("Final RPC transfer transaction completed successfully");
            true
        },
        Err(e) => {
            warn!("Final transaction failed (continuing with measurement): {}", e);
            false
        }
    };
    
    // Always perform measurement regardless of final transaction status
    let actual_completion_time = start_time.elapsed();
    let total_orders = counter.load(Ordering::Relaxed);
    let actual_throughput = total_orders as f64 / actual_completion_time.as_secs_f64();
    
    info!("Spot trading burst test completed!");
    info!("==================================");
    info!("Final transaction status: {}", if final_tx_success { "SUCCESS" } else { "FAILED (measurement still valid)" });
    info!("Total orders sent: {}", total_orders);
    info!("Actual completion time: {:.2} seconds", actual_completion_time.as_secs_f64());
    info!("Actual orders per second: {:.1} orders/s", actual_throughput);

    Ok(())
} 