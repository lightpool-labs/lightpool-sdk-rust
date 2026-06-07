use lightpool_sdk::{
    LightPoolClient, TransactionBuilder, ActionBuilder, Signer,
    Address, ContractAddress, CreateTokenParams, TransferParams, MintParams, ObjectID,
    ExecutionStatus,
    extract_token_address_from_events,
    token_object_id, balance_object_id, TOKEN_SCALE,
    print_receipt_json,
};
use std::time::Duration;
use env_logger::Env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    println!("LightPool SDK Client Example");
    println!("================================");

    let signer = Signer::new();
    let sender_address = signer.address();

    println!("Generated Signer:");
    println!("   Address: {}", sender_address);
    println!("   Public Key: {}", hex::encode(signer.public_key().as_ref()));

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

    println!("\nExample 1: Creating a token");
    println!("-----------------------------");

    let create_params = CreateTokenParams {
        name: "LightPool SDK Token".into(),
        symbol: "LPSDK".into(),
        total_supply: 1_000_000 * TOKEN_SCALE,
        mintable: true,
        to: sender_address,
    };

    let create_action = ActionBuilder::create_token(create_params)?;

    let create_tx = TransactionBuilder::new()
        .sender(sender_address)
        .expiration(u64::MAX)
        .add_action(create_action)
        .build_and_sign_only(&signer)?;

    let mut token_id: Option<ObjectID> = None;
    let mut token_contract_opt: Option<ContractAddress> = None;
    let mut balance_id: Option<ObjectID> = None;

    match client.submit_transaction(create_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);

            if response.receipt.is_success() {
                let token_addr = extract_token_address_from_events(&response.receipt)
                    .expect("Failed to extract token_address from create_action response");
                println!("   Token contract: {}", token_addr);
                token_contract_opt = Some(token_addr);
                token_id = Some(token_object_id(token_addr));
                balance_id = Some(balance_object_id(token_addr, sender_address));
            } else {
                println!("   ERROR: Token creation failed!");
                if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
                    println!("   Error: {}", error_msg);
                }
                return Ok(());
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit transaction: {}", e);
            return Ok(());
        }
    }

    println!("\nExample 2: Mint tokens");
    println!("------------------------");

    let token_obj_id = token_id.expect("Token ID must be available from create_action");
    let token_addr = token_contract_opt.expect("Token contract must be available from create_action");

    let mint_params = MintParams {
        amount: 5_000 * TOKEN_SCALE,
        to: sender_address,
    };

    let mint_action = ActionBuilder::mint_token(
        token_addr,
        token_obj_id,
        mint_params,
    )?;

    let mint_tx = TransactionBuilder::new()
        .sender(sender_address)
        .expiration(u64::MAX)
        .add_action(mint_action)
        .build_and_sign_only(&signer)?;

    match client.submit_transaction(mint_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);

            if !response.receipt.is_success() {
                println!("   ERROR: Token minting failed!");
                if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
                    println!("   Error: {}", error_msg);
                }
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit transaction: {}", e);
        }
    }

    println!("\nExample 3: Transfer tokens");
    println!("----------------------------");

    let balance_obj_id = balance_id.expect("Balance ID must be available from previous operations");
    let token_addr = token_contract_opt.expect("Token contract must be available from create_action");

    let recipient_address = Address::from(0x2048u128);

    let transfer_params = TransferParams {
        to: recipient_address,
        amount: 2048 * TOKEN_SCALE,
    };

    let transfer_action = ActionBuilder::transfer_token(
        token_addr,
        balance_obj_id,
        transfer_params,
    )?;

    let transfer_tx = TransactionBuilder::new()
        .sender(sender_address)
        .expiration(u64::MAX)
        .add_action(transfer_action)
        .build_and_sign_only(&signer)?;

    match client.submit_transaction(transfer_tx).await {
        Ok(response) => {
            print_receipt_json(&response.receipt);

            if !response.receipt.is_success() {
                println!("   ERROR: Token transfer failed!");
                if let ExecutionStatus::Failure(error_msg) = &response.receipt.status {
                    println!("   Error: {}", error_msg);
                }
            }
        }
        Err(e) => {
            println!("   ERROR: Failed to submit transaction: {}", e);
        }
    }

    Ok(())
}
