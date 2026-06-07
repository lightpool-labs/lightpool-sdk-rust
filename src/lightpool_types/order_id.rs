use std::num::ParseIntError;

/// Monotonic per-market order identifier.
pub type OrderId = u64;

/// Parse an order id from decimal or hex (optional 0x prefix).
pub fn parse_order_id(s: &str) -> Result<OrderId, ParseIntError> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() <= 16 && s.chars().all(|c| c.is_ascii_digit()) {
        return s.parse();
    }
    u64::from_str_radix(s, 16)
}
