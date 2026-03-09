//! Shared Solana address parsing helpers.

use heat_core::error::HeatError;
use solana_pubkey::Pubkey;
use std::str::FromStr;

/// Parse a base58-encoded Solana public key.
///
/// Returns a `HeatError::validation` on any input that is not a valid 32-byte
/// base58-encoded public key.
pub fn parse_pubkey(input: &str) -> Result<Pubkey, HeatError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(HeatError::validation(
            "empty_pubkey",
            "Solana public key must not be empty",
        ));
    }
    Pubkey::from_str(trimmed).map_err(|_| {
        HeatError::validation(
            "invalid_pubkey",
            format!("Invalid Solana address: '{trimmed}'"),
        )
        .with_hint("A Solana address is a base58-encoded 32-byte public key (32–44 characters)")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The system program is the canonical well-known Solana address.
    const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

    // A real mainnet address (Serum DEX v3 program).
    const VALID_ADDR: &str = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";

    #[test]
    fn parse_system_program() {
        let pk = parse_pubkey(SYSTEM_PROGRAM).unwrap();
        assert_eq!(pk, Pubkey::default());
    }

    #[test]
    fn parse_valid_address() {
        assert!(parse_pubkey(VALID_ADDR).is_ok());
    }

    #[test]
    fn parse_trims_whitespace() {
        assert!(parse_pubkey(&format!("  {VALID_ADDR}  ")).is_ok());
    }

    #[test]
    fn parse_empty_is_error() {
        let err = parse_pubkey("").unwrap_err();
        assert_eq!(err.reason, "empty_pubkey");
    }

    #[test]
    fn parse_invalid_base58_is_error() {
        // '0', 'O', 'I', 'l' are not valid base58 characters.
        let err = parse_pubkey("0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl").unwrap_err();
        assert_eq!(err.reason, "invalid_pubkey");
    }

    #[test]
    fn parse_too_short_is_error() {
        let err = parse_pubkey("abc123").unwrap_err();
        assert_eq!(err.reason, "invalid_pubkey");
    }

    #[test]
    fn parse_0x_evm_address_is_error() {
        // EVM hex addresses must not be accepted.
        let err = parse_pubkey("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").unwrap_err();
        assert_eq!(err.reason, "invalid_pubkey");
    }
}
