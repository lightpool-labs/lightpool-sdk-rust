use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

const CHARSET: &[u8] = b"_12345abcdefghijklmnopqrstuvwxyz";
const BASE: u64 = 32;
const NAME_LENGTH: usize = 12;
const BITS_PER_CHAR: u32 = 5;
const MAX_REPRESENTABLE_BITS: u32 = NAME_LENGTH as u32 * BITS_PER_CHAR; // 60 bits

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Name(u64);

impl Name {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Const version for compile-time conversion of string literals
    /// This function panics on invalid input to maintain const compatibility
    pub const fn from_str_literal_const(s: &str) -> Self {
        let bytes = s.as_bytes();
        if bytes.len() > NAME_LENGTH {
            panic!("Name literal too long");
        }

        let mut result = 0u64;
        let mut i = 0;
        
        // Process each character
        while i < bytes.len() {
            let digit = match bytes[i] {
                b'_' => 0,
                b'1'..=b'5' => (bytes[i] - b'1' + 1) as u64,
                b'a'..=b'z' => (bytes[i] - b'a' + 6) as u64,
                _ => panic!("Invalid character in name literal"),
            };
            
            result = result * BASE + digit;
            i += 1;
        }
        
        // Pad with zeros if string is shorter than 12 characters
        let mut chars_processed = bytes.len();
        while chars_processed < NAME_LENGTH {
            result = result * BASE;
            chars_processed += 1;
        }
        
        Self(result)
    }

    pub fn from_string(s: &str) -> Result<Self, NameError> {
        if s.len() > NAME_LENGTH {
            return Err(NameError::InvalidLength);
        }

        let mut result = 0u64;
        for c in s.chars() {
            let digit = match c {
                '_' => 0,
                '1'..='5' => (c as u8 - b'1' + 1) as u64,
                'a'..='z' => (c as u8 - b'a' + 6) as u64,
                _ => return Err(NameError::InvalidCharacter),
            };
            
            result = result.checked_mul(BASE)
                .and_then(|r| r.checked_add(digit))
                .ok_or(NameError::Overflow)?;
        }

        // Pad with zeros if string is shorter than 12 characters
        let mut chars_processed = s.len();
        while chars_processed < NAME_LENGTH {
            result = result.checked_mul(BASE)
                .ok_or(NameError::Overflow)?;
            chars_processed += 1;
        }

        Ok(Self(result))
    }

    /// Convert an arbitrary string literal like "transfer" to a Name
    /// This function only accepts valid characters and returns an error for invalid ones
    pub fn from_str_literal(s: &str) -> Result<Self, NameError> {
        if s.len() > NAME_LENGTH {
            return Err(NameError::InvalidLength);
        }

        let mut result = 0u64;
        let mut chars_processed = 0;
        
        for c in s.chars() {
            let digit = match c {
                '_' => 0,
                '1'..='5' => (c as u8 - b'1' + 1) as u64,
                'a'..='z' => (c as u8 - b'a' + 6) as u64,
                _ => return Err(NameError::InvalidCharacter),
            };
            
            result = result.checked_mul(BASE)
                .and_then(|r| r.checked_add(digit))
                .ok_or(NameError::Overflow)?;
            chars_processed += 1;
        }
        
        // Pad with zeros if string is shorter than 12 characters
        while chars_processed < NAME_LENGTH {
            result = result.checked_mul(BASE)
                .ok_or(NameError::Overflow)?;
            chars_processed += 1;
        }
        
        Ok(Self(result))
    }

    pub fn to_string(&self) -> String {
        let mut value = self.0;
        let mut result = Vec::with_capacity(NAME_LENGTH);

        if value == 0 {
            return String::from("_");
        }

        while value > 0 {
            let remainder = (value % BASE) as usize;
            result.push(CHARSET[remainder]);
            value /= BASE;
        }

        result.reverse();
        // Convert to string and trim trailing underscores
        let s = String::from_utf8(result).unwrap();
        s.trim_end_matches('_').to_string()
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn max_representable_value() -> u64 {
        (1u64 << MAX_REPRESENTABLE_BITS) - 1
    }
}

/// Macro to create a Name from a string literal at compile time
#[macro_export]
macro_rules! name {
    ($s:expr) => {
        Name::from_str_literal_const($s)
    };
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for Name {
    type Err = NameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s)
    }
}

impl From<u64> for Name {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<Name> for u64 {
    fn from(name: Name) -> Self {
        name.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    InvalidLength,
    InvalidCharacter,
    Overflow,
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameError::InvalidLength => write!(f, "Name must be exactly {} characters", NAME_LENGTH),
            NameError::InvalidCharacter => write!(f, "Name contains invalid character (must be _, 1-5 or a-z)"),
            NameError::Overflow => write!(f, "Name value too large"),
        }
    }
}

impl std::error::Error for NameError {} 