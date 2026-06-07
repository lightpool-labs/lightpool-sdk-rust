use lightpool_sdk::{
    LightPoolClient, TransactionBuilder, ActionBuilder, Signer,
    Address, U256, ObjectID, CreateTokenParams, TransferParams,
};
use std::time::Duration;

/// Test creating and sending a token creation transaction
#[tokio::test]
async fn test_create_token_transaction() {
    // Create a signer
    let signer = Signer::new();
    let sender_address = signer.address();
    
    // Create token parameters
    let create_params = CreateTokenParams {
        name: "Test Token".to_string(),
        symbol: "TEST".to_string(),
        decimals: 18,
        total_supply: U256::from(1000000u64),
        mintable: true,
    };
    
    // Build the action
    let contract_address = Address::two(); // Use standard address
    let module_address = Address::two();
    let create_action = ActionBuilder::create_token(
        contract_address,
        module_address,
        create_params,
    ).expect("Failed to create token action");
    
    // Build and sign the transaction
    let verified_tx = TransactionBuilder::new()
        .sender(sender_address)
        .nonce(1)
        .gas_budget(1000000)
        .gas_price(1)
        .add_action(create_action)
        .build_and_sign(&signer)
        .expect("Failed to build transaction");
    
    println!("Created token creation transaction:");
    println!("  Sender: {}", sender_address);
    println!("  Digest: {}", hex::encode(verified_tx.digest().as_bytes()));
    println!("  Actions: {}", verified_tx.transaction().actions().len());
    
    // Note: Commented out because we need a running node
    // let client = LightPoolClient::new("http://localhost:8080");
    // let response = client.submit_transaction(verified_tx).await;
    // println!("Transaction response: {:?}", response);
}

/// Test creating a token transfer transaction
#[tokio::test]
async fn test_transfer_token_transaction() {
    // Create signers for sender and recipient
    let sender_signer = Signer::new();
    let sender_address = sender_signer.address();
    let recipient_address = Address::from_suffix(0x999); // Mock recipient
    
    // Create transfer parameters
    let balance_id = ObjectID::random(); // Mock balance ID
    let transfer_params = TransferParams {
        balance_id,
        to: recipient_address,
        amount: U256::from(100u64),
    };
    
    // Build the action
    let contract_address = Address::two();
    let module_address = Address::two();
    let transfer_action = ActionBuilder::transfer_token(
        contract_address,
        module_address,
        transfer_params,
    ).expect("Failed to create transfer action");
    
    // Build and sign the transaction
    let verified_tx = TransactionBuilder::new()
        .sender(sender_address)
        .nonce(2)
        .gas_budget(500000)
        .gas_price(1)
        .add_action(transfer_action)
        .build_and_sign(&sender_signer)
        .expect("Failed to build transaction");
    
    println!("Created token transfer transaction:");
    println!("  Sender: {}", sender_address);
    println!("  Recipient: {}", recipient_address);
    println!("  Digest: {}", hex::encode(verified_tx.digest().as_bytes()));
    println!("  Balance ID: {}", balance_id);
}

/// Test client connectivity (requires running node)
#[tokio::test]
#[ignore] // Ignore by default since it needs a running node
async fn test_client_connectivity() {
    let client = LightPoolClient::new("http://localhost:8080")
        .with_timeout(Duration::from_secs(5));
    
    // Test health check
    match client.health_check().await {
        Ok(healthy) => {
            println!("Node health check: {}", if healthy { "OK" } else { "Failed" });
        }
        Err(e) => {
            println!("Health check error: {}", e);
        }
    }
}

/// Test complete transaction flow (requires running node)
#[tokio::test]
#[ignore] // Ignore by default since it needs a running node
async fn test_complete_transaction_flow() {
    // Create client
    let client = LightPoolClient::new("http://localhost:8080")
        .with_timeout(Duration::from_secs(10));
    
    // Create signer
    let signer = Signer::new();
    let sender_address = signer.address();
    
    println!("Testing complete transaction flow:");
    println!("  Sender address: {}", sender_address);
    
    // Create token creation transaction
    let create_params = CreateTokenParams {
        name: "SDK Test Token".to_string(),
        symbol: "SDKTest".to_string(),
        decimals: 18,
        total_supply: U256::from(1000000u64),
        mintable: true,
    };
    
    let create_action = ActionBuilder::create_token(
        Address::two(),
        Address::two(),
        create_params,
    ).expect("Failed to create token action");
    
    let verified_tx = TransactionBuilder::new()
        .sender(sender_address)
        .nonce(1)
        .gas_budget(1000000)
        .gas_price(1)
        .add_action(create_action)
        .build_and_sign(&signer)
        .expect("Failed to build transaction");
    
    // Submit transaction
    match client.submit_transaction(verified_tx).await {
        Ok(response) => {
            println!("Transaction submitted successfully!");
            println!("  Digest: {}", response.digest);
            println!("  Receipt: {:?}", response.receipt);
        }
        Err(e) => {
            println!("Failed to submit transaction: {}", e);
        }
    }
}

/// Demonstrate SDK usage patterns
#[test]
fn test_sdk_usage_patterns() {
    // Create multiple signers
    let signer1 = Signer::new();
    let signer2 = Signer::new();
    
    println!("SDK Usage Patterns:");
    println!("  Signer 1 Address: {}", signer1.address());
    println!("  Signer 2 Address: {}", signer2.address());
    
    // Export and import private key
    let secret_key_bytes = signer1.export_secret_key_bytes();
    let imported_signer = Signer::from_secret_key_bytes(&secret_key_bytes)
        .expect("Failed to import signer");
    
    assert_eq!(signer1.address(), imported_signer.address());
    println!("  ✓ Private key export/import works correctly");
    
    // Test transaction builder with multiple actions
    let mut builder = TransactionBuilder::new()
        .sender(signer1.address())
        .nonce(10)
        .gas_budget(2000000)
        .gas_price(5);
    
    // Add custom action
    let custom_action = ActionBuilder::custom_action(
        vec![ObjectID::random()],
        Address::two(),
        Address::two(),
        "custom_function",
        b"custom_params".to_vec(),
    );
    
    builder = builder.add_action(custom_action);
    
    let tx = builder.build().expect("Failed to build transaction");
    println!("  ✓ Transaction with {} actions built successfully", tx.actions().len());
} 