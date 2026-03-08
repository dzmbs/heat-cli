use crate::error::HeatError;
use std::fmt;

/// Validate an Ethereum-style hex address (0x + 40 hex chars).
pub fn eth_address(input: &str) -> Result<String, HeatError> {
    let s = input.trim();
    if !s.starts_with("0x") && !s.starts_with("0X") {
        return Err(HeatError::validation(
            "invalid_address",
            format!("Address must start with 0x: {s}"),
        ));
    }
    let hex_part = &s[2..];
    if hex_part.len() != 40 {
        return Err(HeatError::validation(
            "invalid_address",
            format!("Address must be 42 characters (0x + 40 hex): {s}"),
        ));
    }
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(HeatError::validation(
            "invalid_address",
            format!("Address contains non-hex characters: {s}"),
        ));
    }
    Ok(format!("0x{}", hex_part.to_lowercase()))
}

/// A validated decimal amount, stored as a string to preserve precision.
/// Use this for all financial amounts — never f64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amount(String);

impl Amount {
    /// The raw validated string, suitable for passing to protocol SDKs.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl serde::Serialize for Amount {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// Parse and validate a positive decimal amount. Keeps the original string.
pub fn positive_amount(input: &str) -> Result<Amount, HeatError> {
    let s = input.trim();
    validate_decimal_str(s)?;
    if s.starts_with('-') {
        return Err(HeatError::validation(
            "non_positive_amount",
            format!("Amount must be positive: {s}"),
        ));
    }
    if is_zero(s) {
        return Err(HeatError::validation(
            "non_positive_amount",
            format!("Amount must be positive: {s}"),
        ));
    }
    Ok(Amount(s.to_string()))
}

/// Parse and validate a non-negative decimal amount (zero allowed).
pub fn non_negative_amount(input: &str) -> Result<Amount, HeatError> {
    let s = input.trim();
    validate_decimal_str(s)?;
    if s.starts_with('-') {
        return Err(HeatError::validation(
            "negative_amount",
            format!("Amount must not be negative: {s}"),
        ));
    }
    Ok(Amount(s.to_string()))
}

fn validate_decimal_str(s: &str) -> Result<(), HeatError> {
    if s.is_empty() {
        return Err(HeatError::validation("invalid_amount", "Amount is empty"));
    }
    let check = s.strip_prefix('-').unwrap_or(s);
    if check.is_empty() {
        return Err(HeatError::validation(
            "invalid_amount",
            format!("Invalid amount: {s}"),
        ));
    }
    let mut saw_dot = false;
    for c in check.chars() {
        if c == '.' {
            if saw_dot {
                return Err(HeatError::validation(
                    "invalid_amount",
                    format!("Invalid amount: {s}"),
                ));
            }
            saw_dot = true;
        } else if !c.is_ascii_digit() {
            return Err(HeatError::validation(
                "invalid_amount",
                format!("Invalid amount: {s}"),
            ));
        }
    }
    // Must have at least one digit
    if check == "." {
        return Err(HeatError::validation(
            "invalid_amount",
            format!("Invalid amount: {s}"),
        ));
    }
    Ok(())
}

fn is_zero(s: &str) -> bool {
    let check = s.strip_prefix('-').unwrap_or(s);
    check.chars().all(|c| c == '0' || c == '.')
}

/// Validate a network name.
pub fn network_name(input: &str) -> Result<String, HeatError> {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return Err(HeatError::validation(
            "empty_network",
            "Network name cannot be empty",
        ));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(HeatError::validation(
            "invalid_network",
            format!("Invalid network name: {input}"),
        ));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_address() {
        let addr = eth_address("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").unwrap();
        assert_eq!(addr, "0xd8da6bf26964af9d7eed9e03e53415d37aa96045");
    }

    #[test]
    fn test_invalid_address() {
        assert!(eth_address("not-an-address").is_err());
        assert!(eth_address("0x123").is_err());
        assert!(eth_address("0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG").is_err());
    }

    #[test]
    fn test_positive_amount() {
        let a = positive_amount("100.5").unwrap();
        assert_eq!(a.as_str(), "100.5");

        let b = positive_amount("0.001").unwrap();
        assert_eq!(b.as_str(), "0.001");

        // Preserves original precision
        let c = positive_amount("1.00000000").unwrap();
        assert_eq!(c.as_str(), "1.00000000");

        assert!(positive_amount("0").is_err());
        assert!(positive_amount("0.0").is_err());
        assert!(positive_amount("-5").is_err());
        assert!(positive_amount("abc").is_err());
        assert!(positive_amount("").is_err());
        assert!(positive_amount("1.2.3").is_err());
    }

    #[test]
    fn test_non_negative_amount() {
        assert!(non_negative_amount("0").is_ok());
        assert!(non_negative_amount("0.0").is_ok());
        assert!(non_negative_amount("100").is_ok());
        assert!(non_negative_amount("-5").is_err());
    }

    #[test]
    fn test_network_name() {
        assert_eq!(network_name("mainnet").unwrap(), "mainnet");
        assert_eq!(network_name("arbitrum-one").unwrap(), "arbitrum-one");
        assert!(network_name("").is_err());
        assert!(network_name("foo bar").is_err());
    }
}
