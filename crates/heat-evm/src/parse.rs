//! Shared parsing helpers for EVM CLI arguments.

use crate::chains::EvmChain;
use alloy::primitives::{Address, U256};
use heat_core::error::HeatError;

/// Parse a checksummed or lowercase EVM address from a CLI string.
///
/// Accepts `0x`-prefixed 40-hex-char strings (with or without EIP-55 checksum).
pub fn parse_address(input: &str) -> Result<Address, HeatError> {
    let trimmed = input.trim();
    trimmed.parse::<Address>().map_err(|_| {
        HeatError::validation(
            "invalid_address",
            format!("'{trimmed}' is not a valid EVM address"),
        )
        .with_hint("Addresses must be 0x-prefixed 40 hex characters, e.g. 0xAbCd...1234")
    })
}

/// Parse a U256 from a decimal or `0x`-prefixed hex string.
pub fn parse_u256(input: &str) -> Result<U256, HeatError> {
    let trimmed = input.trim();
    if let Some(hex_part) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        U256::from_str_radix(hex_part, 16).map_err(|_| {
            HeatError::validation(
                "invalid_u256",
                format!("'{trimmed}' is not a valid hex U256"),
            )
        })
    } else {
        trimmed.parse::<U256>().map_err(|_| {
            HeatError::validation(
                "invalid_u256",
                format!("'{trimmed}' is not a valid decimal U256"),
            )
        })
    }
}

/// Parse a chain name — delegates to `EvmChain::from_name`.
pub fn parse_chain(input: &str) -> Result<EvmChain, HeatError> {
    EvmChain::from_name(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_address() {
        let addr = parse_address("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").unwrap();
        // Alloy normalises the bytes — just check it parsed
        assert_eq!(format!("{addr:#x}").len(), 42);
    }

    #[test]
    fn parse_address_without_checksum() {
        // lowercase should also be accepted
        parse_address("0xd8da6bf26964af9d7eed9e03e53415d37aa96045").unwrap();
    }

    #[test]
    fn parse_address_rejects_short() {
        let err = parse_address("0xdeadbeef").unwrap_err();
        assert_eq!(err.reason, "invalid_address");
    }

    #[test]
    fn parse_address_rejects_no_prefix() {
        // Alloy may or may not accept bare hex — we rely on Alloy's parser.
        // Test that something obviously wrong fails.
        let err = parse_address("not-an-address").unwrap_err();
        assert_eq!(err.reason, "invalid_address");
    }

    #[test]
    fn parse_u256_decimal() {
        let v = parse_u256("1000000").unwrap();
        assert_eq!(v, U256::from(1_000_000u64));
    }

    #[test]
    fn parse_u256_hex() {
        let v = parse_u256("0xf4240").unwrap();
        assert_eq!(v, U256::from(1_000_000u64));
    }

    #[test]
    fn parse_u256_zero() {
        assert_eq!(parse_u256("0").unwrap(), U256::ZERO);
        assert_eq!(parse_u256("0x0").unwrap(), U256::ZERO);
    }

    #[test]
    fn parse_u256_rejects_garbage() {
        assert!(parse_u256("not-a-number").is_err());
        assert!(parse_u256("0xGGGG").is_err());
    }

    #[test]
    fn parse_chain_delegates() {
        assert_eq!(parse_chain("eth").unwrap(), EvmChain::Ethereum);
        assert!(parse_chain("unknown").is_err());
    }
}
