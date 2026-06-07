use futures_util::{SinkExt, StreamExt};
use log::{debug, warn, error, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Error as WsError;
use url::Url;

use crate::error::{SdkError, SdkResult};
use crate::ws::message::{Message, Subscription};


/// Client for interacting with LightPool WebSocket API
pub struct InfoClient {
    endpoint: String,
    request_id: AtomicU64,
    ws_stream: Arc<Mutex<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
}

impl InfoClient {
    /// Create a new client
    pub async fn new(endpoint: Option<String>) -> SdkResult<Self> {
        let endpoint = endpoint.unwrap_or_else(|| "ws://127.0.0.1:26400".to_string());
        
        let client = Self {
            endpoint,
            request_id: AtomicU64::new(1),
            ws_stream: Arc::new(Mutex::new(None)),
        };
        
        Ok(client)
    }
    
    /// Get the next request ID
    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Connect to the WebSocket endpoint
    async fn connect(&self) -> SdkResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let url = Url::parse(&self.endpoint)
            .map_err(|e| SdkError::Network(format!("Invalid WebSocket URL: {}", e)))?;
        
        let mut request = url.into_client_request()
            .map_err(|e| SdkError::Network(format!("Failed to create WebSocket request: {}", e)))?;
        
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            "jsonrpc".parse().unwrap(),
        );
        
        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|e| SdkError::Network(format!("WebSocket connection failed: {}", e)))?;
        
        info!("Connected to WebSocket endpoint: {}", self.endpoint);
        Ok(ws_stream)
    }
    
    /// Subscribe to updates
    pub async fn subscribe(
        &mut self,
        subscription: Subscription,
        sender: mpsc::UnboundedSender<Message>,
    ) -> SdkResult<String> {
        // Connect if not already connected
        let mut ws_stream_guard = self.ws_stream.lock().await;
        if ws_stream_guard.is_none() {
            *ws_stream_guard = Some(self.connect().await?);
        }
        
        let ws_stream = ws_stream_guard.as_mut().unwrap();
        
        // Create a oneshot channel to receive the subscription ID
        let (id_sender, id_receiver) = oneshot::channel();
        
        // Create subscription request
        let request_id = self.next_request_id();
        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "subscribe",
            "params": subscription
        });

        // Send subscription request
        ws_stream.send(WsMessage::Text(request.to_string()))
            .await
            .map_err(|e| SdkError::Network(format!("Failed to send subscription request: {}", e)))?;

        // Start a task to handle incoming messages
        let ws_stream_clone = self.ws_stream.clone();
        let sender_clone = sender.clone();
        let request_id_for_sub = request_id;
        
        tokio::spawn(async move {
            Self::handle_messages(ws_stream_clone, sender_clone, Some(id_sender), request_id_for_sub).await;
        });

        drop(ws_stream_guard);
        
        // Wait for the subscription ID
        let subscription_id = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            id_receiver
        ).await
            .map_err(|_| SdkError::Timeout("Timed out waiting for subscription confirmation".to_string()))?
            .map_err(|_| SdkError::Network("Failed to receive subscription ID".to_string()))?;
        
        Ok(subscription_id)
    }
    
    /// Unsubscribe from updates
    pub async fn unsubscribe(&mut self, subscription_id: String) -> SdkResult<()> {
        let mut ws_stream_guard = self.ws_stream.lock().await;
        if let Some(ws_stream) = ws_stream_guard.as_mut() {
            // Create unsubscribe request
            let request = json!({
                "jsonrpc": "2.0",
                "id": self.next_request_id(),
                "method": "unsubscribe",
                "params": subscription_id
            });
            
            // Send unsubscribe request
            ws_stream.send(WsMessage::Text(request.to_string()))
                .await
                .map_err(|e| SdkError::Network(format!("Failed to send unsubscribe request: {}", e)))?;
            
            Ok(())
        } else {
            Err(SdkError::Network("WebSocket not connected".to_string()))
        }
    }
    
    /// Handle incoming WebSocket messages
    async fn handle_messages(
        ws_stream: Arc<Mutex<Option<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
        sender: mpsc::UnboundedSender<Message>,
        mut id_sender: Option<oneshot::Sender<String>>,
        request_id: u64,
    ) {
        info!("request_id: {}", request_id);

        loop {
            let mut ws_stream_guard = ws_stream.lock().await;
            let ws_stream_ref = match ws_stream_guard.as_mut() {
                Some(stream) => stream,
                None => {
                    error!("WebSocket stream not available");
                    break;
                }
            };
            
            match ws_stream_ref.next().await {
                Some(Ok(WsMessage::Text(text))) => {
                    //info!("Received WebSocket message: {}", text);
                    
                    match serde_json::from_str::<Value>(&text) {
                        Ok(value) => {
                            // Handle subscription confirmation
                            if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                                if method == "notifications" {
                                    if let Some(params) = value.get("params") {
                                        if let Some(result) = params.get("result") {
                                            if let Some(subscription_id) = result.get("subscription_id").and_then(|s| s.as_str()) {
                                                info!("subscription_id: {:?}", subscription_id);
                                                if let Some(sender) = id_sender.take() {
                                                    let _ = sender.send(subscription_id.to_string());
                                                }
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Handle subscription notifications
                            if let Some(params) = value.get("params") {
                                if let Some(result) = params.get("result") {
                                    if let Some(data) = result.get("data") {
                                        // Server packs a typed enum; try parse full Message first
                                        if let Ok(msg) = serde_json::from_value::<Message>(data.clone()) {
                                            if sender.send(msg).is_err() {
                                                error!("Failed to send WS notification");
                                                break;
                                            }
                                            continue;
                                        }
                                    }
                                }
                            }
                            
                            // Handle error
                            if let Some(error) = value.get("error") {
                                let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                                let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
                                let msg = Message::Error(format!("{}: {}", code, message));
                                if sender.send(msg).is_err() {
                                    error!("Failed to send error notification");
                                    break;
                                }
                                continue;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse WebSocket message: {}", e);
                        }
                    }
                }
                Some(Ok(WsMessage::Close(_))) => {
                    info!("WebSocket connection closed by server");
                    *ws_stream_guard = None;
                    break;
                }
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    *ws_stream_guard = None;
                    break;
                }
                None => {
                    info!("WebSocket connection closed");
                    *ws_stream_guard = None;
                    break;
                }
                _ => {}
            }
        }
    }
} 