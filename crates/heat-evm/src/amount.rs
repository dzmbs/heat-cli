//! Exact decimal ↔ base-unit conversions for ERC-20 / EVM amounts.
//!
//! No f64 is used anywhere — all arithmetic is done with integer and
//! string operations to preserve exactness for financial quantities.

use alloy::primitives::U256;
use heat_core::error::HeatError;

// ── Public API ───────────────────────────────────────────────────────────────

/// Convert a human-readable decimal string into the raw base-unit U256.
///
/// `decimals` is the token's decimal precision (e.g., 6 for USDC, 18 for ETH).
///
/// Examples:
/// ```
/// # use heat_evm::amount::parse_units;
/// # use alloy::primitives::U256;
/// assert_eq!(parse_units("1.5", 6).unwrap(), U256::from(1_500_000u64));
/// assert_eq!(parse_units("1",   0).unwrap(), U256::from(1u64));
/// assert_eq!(parse_units("0",   18).unwrap(), U256::ZERO);
/// ```
pub fn parse_units(input: &str, decimals: u8) -> Result<U256, HeatError> {
    let s = input.trim();
    if s.is_empty() {
        return Err(HeatError::validation(
            "invalid_amount",
            "Amount string is empty",
        ));
    }

    // Reject negative values
    if s.starts_with('-') {
        return Err(HeatError::validation(
            "invalid_amount",
            format!("Amount must be non-negative, got '{s}'"),
        ));
    }

    // Split on the decimal point (allow at most one)
    let (integer_part, fractional_part) = match s.split_once('.') {
        None => (s, ""),
        Some((int, frac)) => {
            if frac.contains('.') {
                return Err(HeatError::validation(
                    "invalid_amount",
                    format!("Amount has more than one decimal point: '{s}'"),
                ));
            }
            (int, frac)
        }
    };

    // Validate characters
    for ch in integer_part.chars().chain(fractional_part.chars()) {
        if !ch.is_ascii_digit() {
            return Err(HeatError::validation(
                "invalid_amount",
                format!("Amount '{s}' contains non-digit character '{ch}'"),
            ));
        }
    }

    // Reject or pad the fractional part to exactly `decimals` digits
    let frac_len = fractional_part.len();
    let decimals_usize = decimals as usize;

    if frac_len > decimals_usize {
        return Err(HeatError::validation(
            "amount_too_precise",
            format!(
                "Amount '{s}' has {frac_len} decimal places but the token only supports {decimals}"
            ),
        ));
    }

    let adjusted_frac: String = if frac_len < decimals_usize {
        // Fewer digits — right-pad with zeros
        format!("{fractional_part:0<decimals_usize$}")
    } else {
        fractional_part.to_string()
    };

    // Concatenate integer + fractional digits to form the raw base-unit string
    let raw = format!("{integer_part}{adjusted_frac}");

    // Strip any leading zeros to avoid false octal parsing (not relevant for U256::from_str,
    // but keeps the logic clean)
    let raw = raw.trim_start_matches('0');
    let raw = if raw.is_empty() { "0" } else { raw };

    raw.parse::<U256>().map_err(|_| {
        HeatError::validation("invalid_amount", format!("Amount '{s}' overflows U256"))
    })
}

/// Format a raw base-unit U256 as a human-readable decimal string.
///
/// `decimals` is the token's decimal precision.
///
/// Examples:
/// ```
/// # use heat_evm::amount::format_units;
/// # use alloy::primitives::U256;
/// assert_eq!(format_units(U256::from(1_500_000u64), 6), "1.500000");
/// assert_eq!(format_units(U256::ZERO, 18), "0.000000000000000000");
/// assert_eq!(format_units(U256::from(1u64), 0), "1");
/// ```
pub fn format_units(value: U256, decimals: u8) -> String {
    if decimals == 0 {
        return value.to_string();
    }

    let s = value.to_string();
    let decimals_usize = decimals as usize;

    if s.len() <= decimals_usize {
        // Value is less than 1 — pad with leading zeros
        let padding = decimals_usize - s.len();
        let frac = format!("{:0>decimals_usize$}", s, decimals_usize = decimals_usize);
        let _ = padding; // consumed by format
        format!("0.{frac}")
    } else {
        let split = s.len() - decimals_usize;
        let (integer, frac) = s.split_at(split);
        format!("{integer}.{frac}")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_units ──────────────────────────────────────────────────────────

    #[test]
    fn parse_integer_with_decimals() {
        assert_eq!(parse_units("1", 6).unwrap(), U256::from(1_000_000u64));
        assert_eq!(parse_units("10", 6).unwrap(), U256::from(10_000_000u64));
        assert_eq!(parse_units("0", 6).unwrap(), U256::ZERO);
    }

    #[test]
    fn parse_decimal_exact() {
        assert_eq!(parse_units("1.5", 6).unwrap(), U256::from(1_500_000u64));
        assert_eq!(parse_units("0.5", 6).unwrap(), U256::from(500_000u64));
        assert_eq!(parse_units("0.000001", 6).unwrap(), U256::from(1u64));
        assert_eq!(
            parse_units("1.000000", 6).unwrap(),
            U256::from(1_000_000u64)
        );
    }

    #[test]
    fn parse_decimal_18() {
        // 1 ETH in wei
        let one_eth = parse_units("1", 18).unwrap();
        assert_eq!(one_eth, U256::from(10u64).pow(U256::from(18u64)));

        // 1.5 ETH
        let one_half_eth = parse_units("1.5", 18).unwrap();
        let expected = U256::from(15u64) * U256::from(10u64).pow(U256::from(17u64));
        assert_eq!(one_half_eth, expected);
    }

    #[test]
    fn parse_rejects_excess_precision() {
        // 6 decimals but user gave 8 — should reject
        let err = parse_units("1.12345678", 6).unwrap_err();
        assert_eq!(err.reason, "amount_too_precise");
    }

    #[test]
    fn parse_zero_decimals() {
        assert_eq!(parse_units("42", 0).unwrap(), U256::from(42u64));
        assert_eq!(parse_units("0", 0).unwrap(), U256::ZERO);
        // Fractional part with 0 decimals — should reject
        let err = parse_units("1.9", 0).unwrap_err();
        assert_eq!(err.reason, "amount_too_precise");
    }

    #[test]
    fn parse_whitespace_trimmed() {
        assert_eq!(parse_units("  1.5  ", 6).unwrap(), U256::from(1_500_000u64));
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(parse_units("", 6).is_err());
        assert!(parse_units("   ", 6).is_err());
    }

    #[test]
    fn parse_rejects_negative() {
        let err = parse_units("-1", 6).unwrap_err();
        assert_eq!(err.reason, "invalid_amount");
    }

    #[test]
    fn parse_rejects_letters() {
        assert!(parse_units("1abc", 6).is_err());
        assert!(parse_units("abc", 6).is_err());
    }

    #[test]
    fn parse_rejects_multiple_dots() {
        assert!(parse_units("1.2.3", 6).is_err());
    }

    // ── format_units ─────────────────────────────────────────────────────────

    #[test]
    fn format_whole_number() {
        assert_eq!(format_units(U256::from(1_000_000u64), 6), "1.000000");
        assert_eq!(format_units(U256::from(10_000_000u64), 6), "10.000000");
    }

    #[test]
    fn format_fractional() {
        assert_eq!(format_units(U256::from(1_500_000u64), 6), "1.500000");
        assert_eq!(format_units(U256::from(500_000u64), 6), "0.500000");
        assert_eq!(format_units(U256::from(1u64), 6), "0.000001");
    }

    #[test]
    fn format_zero() {
        assert_eq!(format_units(U256::ZERO, 6), "0.000000");
        assert_eq!(format_units(U256::ZERO, 18), "0.000000000000000000");
        assert_eq!(format_units(U256::ZERO, 0), "0");
    }

    #[test]
    fn format_zero_decimals() {
        assert_eq!(format_units(U256::from(42u64), 0), "42");
    }

    #[test]
    fn format_18_decimals() {
        let one_eth = U256::from(10u64).pow(U256::from(18u64));
        assert_eq!(format_units(one_eth, 18), "1.000000000000000000");
    }

    // ── Round-trip exactness ─────────────────────────────────────────────────

    #[test]
    fn roundtrip_usdc() {
        let cases = [
            ("0", 6u8),
            ("1", 6),
            ("1.5", 6),
            ("1.000001", 6),
            ("999999.999999", 6),
        ];
        for (input, decimals) in cases {
            let base = parse_units(input, decimals).expect(input);
            let back = format_units(base, decimals);
            // Re-parse to compare numerically (format may pad zeros)
            let base2 = parse_units(&back, decimals).expect(&back);
            assert_eq!(base, base2, "roundtrip failed for '{input}'");
        }
    }

    #[test]
    fn roundtrip_eth() {
        let cases = ["0", "1", "1.5", "0.001", "1000.123456789012345678"];
        for input in cases {
            let base = parse_units(input, 18).expect(input);
            let back = format_units(base, 18);
            let base2 = parse_units(&back, 18).expect(&back);
            assert_eq!(base, base2, "roundtrip failed for '{input}'");
        }
    }
}
