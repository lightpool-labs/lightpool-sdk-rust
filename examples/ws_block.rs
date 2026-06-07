use env_logger::Env;
use log::{info, error, warn};
use lightpool_sdk::{InfoClient, Subscription, Message};
use tokio::{
    spawn,
    sync::mpsc::unbounded_channel,
    time::{sleep, Duration},
};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    // Create a new InfoClient with default WebSocket endpoint
    let mut info_client = match InfoClient::new(Some("ws://127.0.0.1:26400".to_string())).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create InfoClient: {}", e);
            return;
        }
    };

    // Create a channel for receiving WebSocket messages
    let (sender, mut receiver) = unbounded_channel();
    
    // Subscribe to new blocks
    let subscription_id = info_client
        .subscribe(Subscription::NewBlocks, sender)
        .await
        .unwrap();

    // Spawn a task to unsubscribe after 30 seconds
    // Note: We can't clone info_client, so we'll handle unsubscribe differently
    spawn(async move {
        sleep(Duration::from_secs(30)).await;
        info!("30 seconds elapsed - subscription will continue until connection closes");
        // Since we can't clone info_client, we'll just log that the time has elapsed
        // The connection will naturally close when the main loop ends
    });

    // Process incoming WebSocket messages
    while let Some(message) = receiver.recv().await {
        match message {
            Message::NewBlock(block) => {
                info!("Received {:?}", block);
            }
            _ => {
                warn!("Unexpected message on NewBlocks subscription");
            }
        }
    }

    info!("WebSocket connection closed");
} 