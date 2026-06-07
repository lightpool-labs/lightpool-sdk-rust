# LightPool SDK

A Rust SDK for interacting with the LightPool blockchain. This SDK provides a convenient way to construct transactions, sign them, and submit them to the LightPool network.

## Features

- 🔐 **Cryptographic Operations**: Generate keypairs, sign transactions
- 🔧 **Transaction Building**: Fluent API for constructing transactions with various actions
- 🌐 **RPC Client**: HTTP client for communicating with LightPool nodes
- 🪙 **Token Operations**: Built-in support for token creation, transfer, mint, merge, and split
- ⚡ **Async Support**: Full async/await support using Tokio

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
lightpool-sdk = { path = "../lightpool-sdk" }
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

### 1. Create a Signer

```rust
use lightpool_sdk::Signer;

// Generate a new random keypair
let signer = Signer::new();
let address = signer.address();
println!("Address: {}", address);

// Or import from existing private key
let secret_key_bytes = [0u8; 32]; // Your private key bytes
let signer = Signer::from_secret_key_bytes(&secret_key_bytes)?;
```

### 2. Connect to a Node

```rust
use lightpool_sdk::LightPoolClient;
use std::time::Duration;

let client = LightPoolClient::new("http://localhost:8080")
    .with_timeout(Duration::from_secs(30));

// Test connectivity
let is_healthy = client.health_check().await?;
```

### 3. Create and Submit a Transaction

```rust
use lightpool_sdk::{
    TransactionBuilder, ActionBuilder, CreateTokenParams,
    Address, U256,
};

// Create token parameters
let create_params = CreateTokenParams {
    name: "My Token".to_string(),
    symbol: "MYT".to_string(),
    decimals: 18,
    total_supply: U256::from(1_000_000u64),
    mintable: true,
};

// Build the action
let action = ActionBuilder::create_token(
    Address::two(), // Contract address
    Address::two(), // Module address
    create_params,
)?;

// Build and sign the transaction
let verified_tx = TransactionBuilder::new()
    .sender(signer.address())
    .nonce(1)
    .gas_budget(1_000_000)
    .gas_price(1)
    .add_action(action)
    .build_and_sign(&signer)?;

// Submit to the network
let response = client.submit_transaction(verified_tx).await?;
println!("Transaction hash: {}", response.digest);
```

## Supported Actions

### Token Operations

#### Create Token
```rust
let params = CreateTokenParams {
    name: "Test Token".to_string(),
    symbol: "TEST".to_string(),
    decimals: 18,
    total_supply: U256::from(1_000_000u64),
    mintable: true,
};

let action = ActionBuilder::create_token(contract, module, params)?;
```

#### Transfer Token
```rust
let params = TransferParams {
    balance_id: balance_object_id,
    to: recipient_address,
    amount: U256::from(1000u64),
};

let action = ActionBuilder::transfer_token(contract, module, params)?;
```

#### Mint Token
```rust
let params = MintParams {
    token_id: token_object_id,
    amount: U256::from(1000u64),
    to: recipient_address,
};

let action = ActionBuilder::mint_token(contract, module, params)?;
```

#### Merge Token Balances
```rust
let params = MergeParams {
    main_balance_id: main_balance_id,
    other_balance_ids: vec![balance_id_1, balance_id_2],
};

let action = ActionBuilder::merge_token(contract, module, params)?;
```

#### Split Token Balance
```rust
let params = SplitParams {
    balance_id: balance_id,
    amount: U256::from(500u64),
};

let action = ActionBuilder::split_token(contract, module, params)?;
```

### Custom Actions
```rust
let action = ActionBuilder::custom_action(
    vec![input_object_id],
    contract_address,
    module_address,
    "custom_function_name",
    serialized_parameters,
);
```

## Examples

### Basic Token Creation and Transfer

```rust
use lightpool_sdk::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signer = Signer::new();
    let client = LightPoolClient::new("http://localhost:8080");
    
    // Create a token
    let create_params = CreateTokenParams {
        name: "Demo Token".to_string(),
        symbol: "DEMO".to_string(),
        decimals: 18,
        total_supply: U256::from(1_000_000u64),
        mintable: false,
    };
    
    let create_tx = TransactionBuilder::new()
        .sender(signer.address())
        .nonce(1)
        .add_action(ActionBuilder::create_token(
            Address::two(),
            Address::two(),
            create_params,
        )?)
        .build_and_sign(&signer)?;
    
    let response = client.submit_transaction(create_tx).await?;
    println!("Token created: {}", response.digest);
    println!("Receipt: {:?}", response.receipt);
    
    Ok(())
}
```

### Batch Operations

```rust
let batch_tx = TransactionBuilder::new()
    .sender(signer.address())
    .nonce(1)
    .add_action(action1)
    .add_action(action2)
    .add_action(action3)
    .build_and_sign(&signer)?;

let response = client.submit_transaction(batch_tx).await?;
```

## Running the Examples

### Run the simple client example:
```bash
cd crates/lightpool-sdk
cargo run --example simple_client
```

### Run tests:
```bash
# Run unit tests
cargo test

# Run integration tests (requires running node)
cargo test --test integration_test -- --ignored
```

## Error Handling

The SDK uses the `SdkResult<T>` type for error handling:

```rust
use lightpool_sdk::{SdkError, SdkResult};

match client.submit_transaction(verified_tx).await {
    Ok(response) => {
        println!("Transaction submitted successfully!");
        println!("Digest: {}", response.digest);
        println!("Receipt: {:?}", response.receipt);
    },
    Err(SdkError::Network(e)) => println!("Network error: {}", e),
    Err(SdkError::Crypto(e)) => println!("Crypto error: {}", e),
    Err(SdkError::Transaction(e)) => println!("Transaction error: {}", e),
    Err(e) => println!("Other error: {}", e),
}
```

## Development

### Prerequisites
- Rust 1.70+
- A running LightPool node (for integration tests and examples)

### Building
```bash
cargo build
```

### Testing
```bash
# Unit tests
cargo test

# Integration tests (needs running node)
cargo test -- --ignored
```

## License

This project is licensed under the same license as the LightPool project. 