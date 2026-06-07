use reqwest::Client;
use crate::lightpool_types::{SignedTransaction, VerifiedTransaction};
use crate::types::{SubmitTransactionParams, SubmitTransactionResponse, RpcRequest, RpcResponse};
use crate::error::{SdkError, SdkResult};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use log::{info, error};

#[derive(serde::Deserialize)]
struct CallResponse { bytes: Vec<u8> }

/// Client for interacting with LightPool RPC API
pub struct LightPoolClient {
    client: Client,
    endpoint: String,
    request_id: AtomicU64,
}

impl LightPoolClient {
    /// Create a new client
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.into(),
            request_id: AtomicU64::new(1),
        }
    }
    
    /// Get the next request ID
    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Submit a transaction to the network via RPC
    pub async fn submit_transaction(
        &self,
        signed_tx: SignedTransaction,
    ) -> SdkResult<SubmitTransactionResponse> {
        // Create the params struct that matches SubmitTransactionParams
        let params = SubmitTransactionParams { tx: signed_tx };

        // info!("{:?}",&params);
        
        // Create JSON-RPC 2.0 request using positional parameters
        let rpc_request = json!({
            "jsonrpc": "2.0",
            "method": "submitTransaction",
            "params": [params],
            "id": self.next_request_id()
        });
        
        // let rpc_request = match serde_json::to_value(&params) {
        //     Ok(value) => json!({
        //         "jsonrpc": "2.0",
        //         "method": "submitTransaction",
        //         "params": [value],
        //         "id": self.next_request_id()
        //     }),
        //     Err(e) => {
        //         error!("Failed to serialize params: {}", e);
        //         return Err(SdkError::Serialization(e));
        //     }
        // };
        let response = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| SdkError::Reqwest(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(SdkError::Network(format!(
                "HTTP status {}",
                response.status()
            )));
        }
        
        let rpc_response: Value = response
            .json()
            .await
            .map_err(|e| SdkError::Reqwest(e.to_string()))?;
        
        // Handle JSON-RPC 2.0 response format
        if let Some(error) = rpc_response.get("error") {
            let error_code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            let error_message = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
            return Err(SdkError::Rpc(format!(
                "RPC error {}: {}",
                error_code, error_message
            )));
        }
        
        let result = rpc_response.get("result")
            .ok_or_else(|| SdkError::Rpc("No result in RPC response".to_string()))?;
        
        // Parse the result as SubmitTransactionResponse
        let submit_response: SubmitTransactionResponse = serde_json::from_value(result.clone())
            .map_err(|e| SdkError::Serialization(e))?;
        
        Ok(submit_response)
    }
    
    pub async fn call(
        &self,
        signed_tx: SignedTransaction,
    ) -> SdkResult<Vec<u8>> {
        let params = SubmitTransactionParams { tx: signed_tx };
        let rpc_request = json!({
            "jsonrpc": "2.0",
            "method": "call",
            "params": [params],
            "id": self.next_request_id()
        });

        let response = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&rpc_request)
            .send()
            .await
            .map_err(|e| SdkError::Reqwest(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SdkError::Network(format!(
                "HTTP status {}",
                response.status()
            )));
        }

        let rpc_response: Value = response
            .json()
            .await
            .map_err(|e| SdkError::Reqwest(e.to_string()))?;

        if let Some(error) = rpc_response.get("error") {
            let error_code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            let error_message = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
            return Err(SdkError::Rpc(format!(
                "RPC error {}: {}",
                error_code, error_message
            )));
        }

        let result = rpc_response.get("result")
            .ok_or_else(|| SdkError::Rpc("No result in RPC response".to_string()))?;

        let call_response: CallResponse = serde_json::from_value(result.clone())
            .map_err(|e| SdkError::Serialization(e))?;

        Ok(call_response.bytes)
    }
    
    /// Set timeout for HTTP requests
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        self
    }
    
    /// Check if the node is reachable
    pub async fn health_check(&self) -> SdkResult<bool> {
        // For JSON-RPC servers, we can try a simple RPC call or check if the endpoint responds
        let ping_request = json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "params": {},
            "id": self.next_request_id()
        });
        
        let response = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&ping_request)
            .send()
            .await;
        
        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

impl Clone for LightPoolClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            endpoint: self.endpoint.clone(),
            request_id: AtomicU64::new(self.request_id.load(Ordering::SeqCst)),
        }
    }
} 