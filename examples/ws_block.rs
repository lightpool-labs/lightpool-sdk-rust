use env_logger::Env;
use lightpool_sdk::{Message, Subscription, WebSocketClient};
use log::{error, info, warn};
use tokio::sync::mpsc::unbounded_channel;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let mut ws_client = match WebSocketClient::new(Some("ws://127.0.0.1:26400".to_string())).await
    {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create WebSocketClient: {}", e);
            return;
        }
    };

    let (sender, mut receiver) = unbounded_channel();

    let subscription_id = ws_client
        .subscribe(Subscription::NewBlocks, sender)
        .await
        .unwrap();

    info!("Subscribed to NewBlocks: {subscription_id}");

    while let Some(message) = receiver.recv().await {
        match message {
            Message::NewBlock(block) => {
                info!("Received block {}", block.block_num);
            }
            Message::Error(err) => {
                error!("WebSocket error: {err}");
                break;
            }
        }
    }

    warn!("WebSocket connection closed");
}
