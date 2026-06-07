use thiserror::Error;

pub type SdkResult<T> = Result<T, SdkError>;

#[derive(Error, Debug)]
pub enum SdkError {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Crypto error: {0}")]
    Crypto(String),
    
    #[error("Transaction error: {0}")]
    Transaction(String),
    
    #[error("RPC error: {0}")]
    Rpc(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(#[from] hex::FromHexError),
    
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),
    
    #[error("Generic error: {0}")]
    Generic(#[from] anyhow::Error),
    
    #[error("Reqwest error: {0}")]
    Reqwest(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
} 