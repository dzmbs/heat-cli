//! Asset resolution: "BTC" → perp, "PURR/USDC" → spot, "dex:BTC" → HIP-3 perp.
//! Adapted from hypecli/src/utils.rs and hlz asset resolution.

use heat_core::error::HeatError;
use hypersdk::hypercore::HttpClient;
use rust_decimal::Decimal;
use strsim::levenshtein;

/// Resolved asset with its index and size decimals.
pub struct ResolvedAsset {
    pub name: String,
    pub index: usize,
    pub sz_decimals: i64,
}

/// Resolve an asset name to its index.
/// Formats: "BTC" (perp), "PURR/USDC" (spot), "dex:BTC" (HIP-3 perp).
pub async fn resolve(client: &HttpClient, input: &str) -> Result<ResolvedAsset, HeatError> {
    let input = input.trim();

    if let Ok(idx) = input.parse::<usize>() {
        return resolve_by_index(client, idx).await;
    }

    if input.contains('/') {
        return resolve_spot(client, input).await;
    }

    if input.contains(':') {
        return resolve_hip3(client, input).await;
    }

    resolve_perp(client, input).await
}

async fn resolve_perp(client: &HttpClient, symbol: &str) -> Result<ResolvedAsset, HeatError> {
    let perps = client.perps().await.map_err(|e| {
        HeatError::network("perps_fetch", format!("Failed to fetch perp markets: {e}"))
    })?;

    let upper = symbol.to_uppercase();
    for p in &perps {
        if p.name.to_uppercase() == upper {
            return Ok(ResolvedAsset {
                name: p.name.clone(),
                index: p.index,
                sz_decimals: p.sz_decimals,
            });
        }
    }

    let names: Vec<&str> = perps.iter().map(|p| p.name.as_str()).collect();
    let suggestion = suggest_similar(symbol, &names);
    Err(HeatError::validation(
        "asset_not_found",
        format!("Perp market not found: {symbol}"),
    )
    .with_hint(suggestion))
}

async fn resolve_spot(client: &HttpClient, input: &str) -> Result<ResolvedAsset, HeatError> {
    let parts: Vec<&str> = input.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(HeatError::validation(
            "invalid_spot_format",
            format!("Invalid spot format: {input}. Expected BASE/QUOTE"),
        ));
    }
    let (base, quote) = (parts[0].to_uppercase(), parts[1].to_uppercase());

    let spots = client.spot().await.map_err(|e| {
        HeatError::network("spot_fetch", format!("Failed to fetch spot markets: {e}"))
    })?;

    for s in &spots {
        if s.tokens[0].name.to_uppercase() == base && s.tokens[1].name.to_uppercase() == quote {
            return Ok(ResolvedAsset {
                name: s.name.clone(),
                index: s.index,
                sz_decimals: s.tokens[0].sz_decimals,
            });
        }
    }

    Err(HeatError::validation(
        "asset_not_found",
        format!("Spot market not found: {input}"),
    )
    .with_hint("Use 'heat hl spot' to list available spot markets"))
}

async fn resolve_hip3(client: &HttpClient, input: &str) -> Result<ResolvedAsset, HeatError> {
    let parts: Vec<&str> = input.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(HeatError::validation(
            "invalid_hip3_format",
            format!("Invalid HIP-3 format: {input}. Expected DEX:SYMBOL"),
        ));
    }
    let (dex_name, symbol) = (parts[0], parts[1].to_uppercase());

    let dexes = client.perp_dexs().await.map_err(|e| {
        HeatError::network("dex_fetch", format!("Failed to fetch DEXes: {e}"))
    })?;

    let dex = dexes
        .iter()
        .find(|d| d.name().eq_ignore_ascii_case(dex_name))
        .ok_or_else(|| {
            let names: Vec<&str> = dexes.iter().map(|d| d.name()).collect();
            HeatError::validation(
                "dex_not_found",
                format!("DEX not found: {dex_name}"),
            )
            .with_hint(suggest_similar(dex_name, &names))
        })?;

    let perps = client.perps_from(dex.clone()).await.map_err(|e| {
        HeatError::network("dex_perps_fetch", format!("Failed to fetch DEX perps: {e}"))
    })?;

    for p in &perps {
        if p.name.to_uppercase() == symbol {
            return Ok(ResolvedAsset {
                name: format!("{}:{}", dex_name, p.name),
                index: p.index,
                sz_decimals: p.sz_decimals,
            });
        }
    }

    Err(HeatError::validation(
        "asset_not_found",
        format!("Market not found on {dex_name}: {symbol}"),
    ))
}

async fn resolve_by_index(client: &HttpClient, idx: usize) -> Result<ResolvedAsset, HeatError> {
    if idx < 10000 {
        let perps = client.perps().await.map_err(|e| {
            HeatError::network("perps_fetch", format!("Failed to fetch perps: {e}"))
        })?;
        for p in &perps {
            if p.index == idx {
                return Ok(ResolvedAsset {
                    name: p.name.clone(),
                    index: p.index,
                    sz_decimals: p.sz_decimals,
                });
            }
        }
    } else {
        let spots = client.spot().await.map_err(|e| {
            HeatError::network("spot_fetch", format!("Failed to fetch spots: {e}"))
        })?;
        for s in &spots {
            if s.index == idx {
                return Ok(ResolvedAsset {
                    name: s.name.clone(),
                    index: s.index,
                    sz_decimals: s.tokens[0].sz_decimals,
                });
            }
        }
    }

    Err(HeatError::validation(
        "asset_not_found",
        format!("No market with index {idx}"),
    ))
}

/// Suggest similar asset names using Levenshtein distance.
fn suggest_similar(input: &str, candidates: &[&str]) -> String {
    let upper = input.to_uppercase();
    let mut scored: Vec<(&str, usize)> = candidates
        .iter()
        .map(|c| (*c, levenshtein(&upper, &c.to_uppercase())))
        .filter(|(_, d)| *d <= 3)
        .collect();
    scored.sort_by_key(|(_, d)| *d);
    scored.truncate(3);

    if scored.is_empty() {
        "Use 'heat hl perps' or 'heat hl spot' to list available markets".to_string()
    } else {
        let suggestions: Vec<&str> = scored.iter().map(|(n, _)| *n).collect();
        format!("Did you mean: {}?", suggestions.join(", "))
    }
}

/// Truncate a decimal to N decimal places (floor, never round up).
/// Adapted from hlz truncDp pattern.
pub fn truncate_size(size: Decimal, sz_decimals: i64) -> Decimal {
    if sz_decimals <= 0 {
        return size.trunc();
    }
    let factor = Decimal::from(10i64.pow(sz_decimals as u32));
    (size * factor).trunc() / factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn truncate_btc_5_decimals() {
        // BTC sz_decimals = 5
        assert_eq!(truncate_size(dec!(0.123456789), 5), dec!(0.12345));
    }

    #[test]
    fn truncate_floors_not_rounds() {
        assert_eq!(truncate_size(dec!(0.99999), 3), dec!(0.999));
        assert_eq!(truncate_size(dec!(1.9999), 2), dec!(1.99));
    }

    #[test]
    fn truncate_zero_decimals() {
        // DOGE sz_decimals = 0
        assert_eq!(truncate_size(dec!(1234.567), 0), dec!(1234));
    }

    #[test]
    fn truncate_negative_decimals() {
        assert_eq!(truncate_size(dec!(1234.567), -1), dec!(1234));
    }

    #[test]
    fn suggest_similar_finds_close_matches() {
        let candidates = ["BTC", "ETH", "SOL", "DOGE"];
        let result = suggest_similar("btcc", &candidates);
        assert!(result.contains("BTC"), "Expected BTC suggestion, got: {result}");
    }

    #[test]
    fn suggest_similar_no_match() {
        let candidates = ["BTC", "ETH"];
        let result = suggest_similar("XXXXXXXX", &candidates);
        assert!(result.contains("heat hl"), "Expected fallback hint, got: {result}");
    }
}
