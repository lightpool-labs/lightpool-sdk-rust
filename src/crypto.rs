use crate::lightpool_types::{generate_production_keypair, PublicKey, SecretKey, Signature, Digest, derive_public_key_from_secret};
use crate::lightpool_types::Address;
use crate::error::{SdkError, SdkResult};
use base64;

/// Signer for creating signatures
pub struct Signer {
    public_key: PublicKey,
    secret_key: SecretKey,
}

impl Signer {
    /// Create a new signer with a random keypair
    pub fn new() -> Self {
        let (public_key, secret_key) = generate_production_keypair();
        Self { public_key, secret_key }
    }
    
    /// Create a signer from existing keys
    pub fn from_keys(public_key: PublicKey, secret_key: SecretKey) -> Self {
        Self { public_key, secret_key }
    }
    
    /// Create a signer from a secret key (derives public key)
    pub fn from_secret_key(secret_key: SecretKey) -> Self {
        let public_key = derive_public_key_from_secret(&secret_key)
            .expect("Failed to derive public key from secret key");
        Self { public_key, secret_key }
    }
    
    /// Get the public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    
    /// Get the address derived from the public key
    pub fn address(&self) -> Address {
        Address::from_public_key(&self.public_key)
    }
    
    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> SdkResult<Signature> {
        let digest = Digest::new_from_bytes(message);
        let signature = Signature::new(&digest, &self.secret_key);
        Ok(signature)
    }
    
    /// Sign a transaction digest
    pub fn sign_transaction(&self, digest: &Digest) -> SdkResult<Signature> {
        let signature = Signature::new(digest, &self.secret_key);
        Ok(signature)
    }
    
    /// Export the secret key as base64 string
    pub fn export_secret_key(&self) -> String {
        self.secret_key.encode_base64()
    }
    
    /// Export the secret key as bytes (for compatibility) - SecretKey is 64 bytes
    pub fn export_secret_key_bytes(&self) -> [u8; 64] {
        // Get the base64 encoded secret key and decode it back to bytes
        let encoded = self.secret_key.encode_base64();
        let decoded = base64::decode(&encoded).unwrap_or_default();
        let mut bytes = [0u8; 64];
        if decoded.len() >= 64 {
            bytes.copy_from_slice(&decoded[..64]);
        }
        bytes
    }
    
    /// Import a signer from secret key bytes (64 bytes for Ed25519)
    pub fn from_secret_key_bytes(bytes: &[u8; 64]) -> SdkResult<Self> {
        // Convert bytes to base64 and then to SecretKey
        let encoded = base64::encode(bytes);
        let secret_key = SecretKey::decode_base64(&encoded)
            .map_err(|e| SdkError::Crypto(format!("Invalid secret key: {}", e)))?;
        Ok(Self::from_secret_key(secret_key))
    }
    
    /// Import a signer from base64 encoded secret key
    pub fn from_secret_key_base64(encoded: &str) -> SdkResult<Self> {
        let secret_key = SecretKey::decode_base64(encoded)
            .map_err(|e| SdkError::Crypto(format!("Invalid secret key: {}", e)))?;
        Ok(Self::from_secret_key(secret_key))
    }
}

impl Default for Signer {
    fn default() -> Self {
        Self::new()
    }
} 