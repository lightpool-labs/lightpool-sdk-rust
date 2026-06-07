use env_logger::Env;
use log::{info, error, warn};
use lightpool_sdk::{InfoClient, Subscription, Message, Address};
use tokio::{
    spawn,
    sync::mpsc::unbounded_channel,
    time::{sleep, Duration},
};
use std::str::FromStr;

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

    let market_addr = match std::env::args().nth(1) {
        Some(s) => match Address::from_str(&s) {
            Ok(addr) => addr,
            Err(_) => {
                error!("Invalid address format. Usage: ws_trades <ADDRESS_HEX>");
                return;
            }
        },
        None => {
            error!("Missing address. Usage: ws_trades <ADDRESS_HEX>");
            return;
        }
    };

    // Subscribe to market trades
    let _subscription_id = info_client
        .subscribe(Subscription::Trades(market_addr), sender)
        .await
        .unwrap();

    // Keep the example running for a while
    spawn(async move {
        sleep(Duration::from_secs(30)).await;
        info!("30 seconds elapsed - subscription will continue until connection closes");
    });

    // Process incoming WebSocket messages
    while let Some(message) = receiver.recv().await {
        match message {
            Message::Trades(trades) => {
                info!("Received trades: {:?}", trades);
            }
            _ => {
                warn!("Unexpected message on Trades subscription");
            }
        }
    }

    info!("WebSocket connection closed");
} 