//! Exact lamport / token-base-unit ↔ decimal conversions.
//!
//! All arithmetic is done in integer arithmetic using `u128` to avoid any
//! floating-point rounding.  `f64` is intentionally absent from this module.
//!
//! Common decimals:
//! - SOL  — 9  decimals (1 SOL = 1_000_000_000 lamports)
//! - USDC — 6  decimals (1 USDC = 1_000_000 micro-USDC)

use heat_core::error::HeatError;

/// Parse a decimal string into base units (lamports / micro-tokens).
///
/// `decimals` is the token's decimal precision (9 for SOL, 6 for USDC, etc.).
///
/// Examples:
/// ```text
/// parse_units("1",       9) == Ok(1_000_000_000)   // 1 SOL
/// parse_units("0.5",     9) == Ok(  500_000_000)   // 0.5 SOL
/// parse_units("1.5",     6) == Ok(    1_500_000)   // 1.5 USDC
/// parse_units("0.000001",6) == Ok(            1)   // 1 micro-USDC
/// ```
///
/// Returns a `HeatError::validation` for:
/// - Empty input
/// - More than one decimal point
/// - Non-digit characters (other than the decimal point)
/// - Fractional part longer than `decimals` digits (would lose precision)
/// - Values that overflow `u64`
pub fn parse_units(input: &str, decimals: u8) -> Result<u64, HeatError> {
    let s = input.trim();
    if s.is_empty() {
        return Err(HeatError::validation(
            "empty_amount",
            "Amount must not be empty",
        ));
    }

    // Split on '.'.
    let (int_part, frac_part) = match s.splitn(2, '.').collect::<Vec<_>>().as_slice() {
        [i] => (*i, ""),
        [i, f] => (*i, *f),
        _ => {
            return Err(HeatError::validation(
                "invalid_amount",
                format!("Invalid amount: '{s}'"),
            ));
        }
    };

    // Validate no extra dots.
    if s.matches('.').count() > 1 {
        return Err(HeatError::validation(
            "invalid_amount",
            format!("Invalid amount (multiple decimal points): '{s}'"),
        ));
    }

    // Validate characters.
    if !int_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(HeatError::validation(
            "invalid_amount",
            format!("Invalid amount (non-digit characters): '{s}'"),
        ));
    }
    if !frac_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(HeatError::validation(
            "invalid_amount",
            format!("Invalid amount (non-digit characters in fractional part): '{s}'"),
        ));
    }

    // Fractional part must not exceed the token's decimal precision.
    let frac_len = frac_part.len();
    if frac_len > decimals as usize {
        return Err(HeatError::validation(
            "amount_too_precise",
            format!(
                "Amount '{s}' has {frac_len} decimal places but the token only supports {decimals}"
            ),
        ));
    }

    // Scale integer part.
    let scale: u128 = 10u128.pow(decimals as u32);
    let int_value: u128 = if int_part.is_empty() {
        0
    } else {
        int_part.parse::<u128>().map_err(|_| {
            HeatError::validation("amount_overflow", format!("Amount too large: '{s}'"))
        })?
    };
    let int_scaled = int_value.checked_mul(scale).ok_or_else(|| {
        HeatError::validation("amount_overflow", format!("Amount too large: '{s}'"))
    })?;

    // Scale fractional part (right-pad with zeros to `decimals` digits).
    let frac_scaled: u128 = if frac_part.is_empty() {
        0
    } else {
        let padding = decimals as usize - frac_len;
        let frac_extended = format!("{frac_part}{}", "0".repeat(padding));
        frac_extended.parse::<u128>().map_err(|_| {
            HeatError::validation("amount_overflow", format!("Amount too large: '{s}'"))
        })?
    };

    let total = int_scaled.checked_add(frac_scaled).ok_or_else(|| {
        HeatError::validation("amount_overflow", format!("Amount too large: '{s}'"))
    })?;

    u64::try_from(total).map_err(|_| {
        HeatError::validation(
            "amount_overflow",
            format!("Amount too large for u64: '{s}'"),
        )
    })
}

/// Format base units as a human-readable decimal string.
///
/// `decimals` is the token's decimal precision (9 for SOL, 6 for USDC, etc.).
/// Trailing zeros in the fractional part are stripped.
///
/// Examples:
/// ```text
/// format_units(1_000_000_000, 9) == "1"
/// format_units(  500_000_000, 9) == "0.5"
/// format_units(    1_500_000, 6) == "1.5"
/// format_units(            1, 6) == "0.000001"
/// format_units(            0, 9) == "0"
/// ```
pub fn format_units(value: u64, decimals: u8) -> String {
    if decimals == 0 {
        return value.to_string();
    }

    let scale = 10u64.pow(decimals as u32);
    let int_part = value / scale;
    let frac_part = value % scale;

    if frac_part == 0 {
        return int_part.to_string();
    }

    // Format fractional part with leading zeros, then strip trailing zeros.
    let frac_str = format!("{frac_part:0>width$}", width = decimals as usize);
    let frac_trimmed = frac_str.trim_end_matches('0');
    format!("{int_part}.{frac_trimmed}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_units ──────────────────────────────────────────────────────────

    #[test]
    fn sol_whole_number() {
        assert_eq!(parse_units("1", 9).unwrap(), 1_000_000_000);
    }

    #[test]
    fn sol_half() {
        assert_eq!(parse_units("0.5", 9).unwrap(), 500_000_000);
    }

    #[test]
    fn sol_min_lamport() {
        assert_eq!(parse_units("0.000000001", 9).unwrap(), 1);
    }

    #[test]
    fn usdc_whole() {
        assert_eq!(parse_units("1", 6).unwrap(), 1_000_000);
    }

    #[test]
    fn usdc_half() {
        assert_eq!(parse_units("1.5", 6).unwrap(), 1_500_000);
    }

    #[test]
    fn usdc_min_unit() {
        assert_eq!(parse_units("0.000001", 6).unwrap(), 1);
    }

    #[test]
    fn zero() {
        assert_eq!(parse_units("0", 9).unwrap(), 0);
    }

    #[test]
    fn zero_decimal() {
        assert_eq!(parse_units("0.0", 9).unwrap(), 0);
    }

    #[test]
    fn no_leading_digit() {
        // ".5" — no integer part
        assert_eq!(parse_units(".5", 9).unwrap(), 500_000_000);
    }

    #[test]
    fn too_many_decimals() {
        // USDC has 6 decimals; 7 fractional digits is too precise.
        let err = parse_units("1.0000001", 6).unwrap_err();
        assert_eq!(err.reason, "amount_too_precise");
    }

    #[test]
    fn empty_is_error() {
        let err = parse_units("", 9).unwrap_err();
        assert_eq!(err.reason, "empty_amount");
    }

    #[test]
    fn invalid_chars_are_error() {
        let err = parse_units("1abc", 9).unwrap_err();
        assert_eq!(err.reason, "invalid_amount");
    }

    #[test]
    fn multiple_dots_are_error() {
        let err = parse_units("1.0.0", 9).unwrap_err();
        assert_eq!(err.reason, "invalid_amount");
    }

    #[test]
    fn large_but_valid() {
        // u64::MAX / 10^9 ~ 18.4 billion SOL — well within u64.
        assert!(parse_units("1000000000", 9).is_ok());
    }

    // ── format_units ─────────────────────────────────────────────────────────

    #[test]
    fn format_sol_whole() {
        assert_eq!(format_units(1_000_000_000, 9), "1");
    }

    #[test]
    fn format_sol_half() {
        assert_eq!(format_units(500_000_000, 9), "0.5");
    }

    #[test]
    fn format_sol_one_lamport() {
        assert_eq!(format_units(1, 9), "0.000000001");
    }

    #[test]
    fn format_usdc_whole() {
        assert_eq!(format_units(1_000_000, 6), "1");
    }

    #[test]
    fn format_usdc_half() {
        assert_eq!(format_units(1_500_000, 6), "1.5");
    }

    #[test]
    fn format_usdc_min() {
        assert_eq!(format_units(1, 6), "0.000001");
    }

    #[test]
    fn format_zero() {
        assert_eq!(format_units(0, 9), "0");
    }

    #[test]
    fn format_no_decimals() {
        assert_eq!(format_units(42, 0), "42");
    }

    // ── roundtrip ────────────────────────────────────────────────────────────

    #[test]
    fn roundtrip_sol() {
        let cases: &[(&str, u64)] = &[
            ("0", 0),
            ("1", 1_000_000_000),
            ("0.5", 500_000_000),
            ("1.23456789", 1_234_567_890),
            ("0.000000001", 1),
        ];
        for (s, units) in cases {
            assert_eq!(parse_units(s, 9).unwrap(), *units, "parse '{s}'");
            assert_eq!(&format_units(*units, 9), s, "format {units}");
        }
    }

    #[test]
    fn roundtrip_usdc() {
        let cases: &[(&str, u64)] = &[
            ("0", 0),
            ("1", 1_000_000),
            ("1.5", 1_500_000),
            ("0.000001", 1),
            ("999999.999999", 999_999_999_999),
        ];
        for (s, units) in cases {
            assert_eq!(parse_units(s, 6).unwrap(), *units, "parse '{s}'");
            assert_eq!(&format_units(*units, 6), s, "format {units}");
        }
    }
}
