use ed25519_dalek as dalek;
use ed25519_dalek::ed25519;
use ed25519_dalek::Signer as _;
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use serde::{de, ser, Deserialize, Serialize};
use std::array::TryFromSliceError;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use bincode;
use hex;

// Re-export needed types
pub use ed25519_dalek::Digest as DalekDigest;
pub use ed25519_dalek::Sha512;

pub type CryptoError = ed25519::Error;

/// Represents a hash digest (32 bytes).
#[derive(Hash, PartialEq, Default, Eq, Clone, Deserialize, Serialize, Ord, PartialOrd, Copy)]
pub struct Digest(pub [u8; 32]);

impl Digest {
    /// Use Sha512 to hash input bytes and create a new Digest
    /// Note: Since Sha512 produces 64 bytes output, we only take the first 32 bytes
    pub fn new_from_bytes(bytes: &[u8]) -> Self {
        let hash = Sha512::digest(bytes);
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_slice()[0..32]);
        Self(result)
    }
    
    /// Create digest from serializable data
    pub fn from_data<T: Serialize>(data: &T) -> Self {
        let bytes = bincode::serialize(data).expect("Failed to serialize data for digest");
        Self::new_from_bytes(&bytes)
    }
    
    /// Generate a random Digest
    #[cfg(feature = "randomization")]
    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }
    
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }
    
    /// Return reference to digest byte array
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create a new digest from raw bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Format digest as hexadecimal string
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(&self.0))
    }

    /// Format digest as short hexadecimal string (first 16 characters)
    pub fn to_hex_short(&self) -> String {
        format!("0x{}", &hex::encode(&self.0)[..16])
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "0x{}", hex::encode(&self.0))
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "0x{}", hex::encode(&self.0))
    }
}

impl AsRef<[u8]> for Digest {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<&[u8]> for Digest {
    type Error = TryFromSliceError;
    fn try_from(item: &[u8]) -> Result<Self, Self::Error> {
        Ok(Digest(item.try_into()?))
    }
}

/// This trait is implemented by all messages that can be hashed.
pub trait Hash {
    fn digest(&self) -> Digest;
}

/// Represents a public key (in bytes).
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    pub fn encode_base64(&self) -> String {
        base64::encode(&self.0[..])
    }

    pub fn decode_base64(s: &str) -> Result<Self, base64::DecodeError> {
        let bytes = base64::decode(s)?;
        let array = bytes[..32]
            .try_into()
            .map_err(|_| base64::DecodeError::InvalidLength)?;
        Ok(Self(array))
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.encode_base64())
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.encode_base64().get(0..16).unwrap())
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.encode_base64())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let value = Self::decode_base64(&s).map_err(|e| de::Error::custom(e.to_string()))?;
        Ok(value)
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Represents a secret key (in bytes).
pub struct SecretKey([u8; 64]);

impl SecretKey {
    pub fn encode_base64(&self) -> String {
        base64::encode(&self.0[..])
    }

    pub fn decode_base64(s: &str) -> Result<Self, base64::DecodeError> {
        let bytes = base64::decode(s)?;
        let array = bytes[..64]
            .try_into()
            .map_err(|_| base64::DecodeError::InvalidLength)?;
        Ok(Self(array))
    }
}

impl Serialize for SecretKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.encode_base64())
    }
}

impl<'de> Deserialize<'de> for SecretKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let value = Self::decode_base64(&s).map_err(|e| de::Error::custom(e.to_string()))?;
        Ok(value)
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        self.0.iter_mut().for_each(|x| *x = 0);
    }
}

pub fn generate_production_keypair() -> (PublicKey, SecretKey) {
    generate_keypair(&mut OsRng)
}

pub fn generate_keypair<R>(csprng: &mut R) -> (PublicKey, SecretKey)
where
    R: CryptoRng + RngCore,
{
    let keypair = dalek::Keypair::generate(csprng);
    let public = PublicKey(keypair.public.to_bytes());
    let secret = SecretKey(keypair.to_bytes());
    (public, secret)
}

/// Derive public key from secret key
pub fn derive_public_key_from_secret(secret: &SecretKey) -> Result<PublicKey, CryptoError> {
    let keypair = dalek::Keypair::from_bytes(&secret.0)?;
    Ok(PublicKey(keypair.public.to_bytes()))
}

/// Represents an ed25519 signature.
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Signature {
    part1: [u8; 32],
    part2: [u8; 32],
}

impl Signature {
    pub fn new(digest: &Digest, secret: &SecretKey) -> Self {
        let keypair = dalek::Keypair::from_bytes(&secret.0).expect("Unable to load secret key");
        let sig = keypair.sign(&digest.0).to_bytes();
        let part1 = sig[..32].try_into().expect("Unexpected signature length");
        let part2 = sig[32..64].try_into().expect("Unexpected signature length");
        Signature { part1, part2 }
    }

    fn flatten(&self) -> [u8; 64] {
        [self.part1, self.part2]
            .concat()
            .try_into()
            .expect("Unexpected signature length")
    }

    pub fn verify(&self, digest: &Digest, public_key: &PublicKey) -> Result<(), CryptoError> {
        let signature = ed25519::signature::Signature::from_bytes(&self.flatten())?;
        let key = dalek::PublicKey::from_bytes(&public_key.0)?;
        key.verify_strict(&digest.0, &signature)
    }

    pub fn verify_batch<'a, I>(digest: &Digest, votes: I) -> Result<(), CryptoError>
    where
        I: IntoIterator<Item = &'a (PublicKey, Signature)>,
    {
        let mut messages: Vec<&[u8]> = Vec::new();
        let mut signatures: Vec<dalek::Signature> = Vec::new();
        let mut keys: Vec<dalek::PublicKey> = Vec::new();
        for (key, sig) in votes.into_iter() {
            messages.push(&digest.0[..]);
            signatures.push(ed25519::signature::Signature::from_bytes(&sig.flatten())?);
            keys.push(dalek::PublicKey::from_bytes(&key.0)?);
        }
        dalek::verify_batch(&messages[..], &signatures[..], &keys[..])
    }
} 