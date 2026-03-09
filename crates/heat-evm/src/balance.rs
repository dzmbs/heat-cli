//! Native and ERC-20 balance queries.

use alloy::primitives::{Address, U256, address};
use alloy::providers::Provider;
use heat_core::error::HeatError;

use crate::chains::EvmChain;
use crate::erc20;

/// Fetch the native gas token balance for an address.
pub async fn native_balance(provider: impl Provider, owner: Address) -> Result<U256, HeatError> {
    provider
        .get_balance(owner)
        .await
        .map_err(|e| HeatError::network("native_balance", format!("Failed to fetch balance: {e}")))
}

/// Fetch an ERC-20 token balance, also returning symbol and decimals.
pub async fn token_balance(
    provider: impl Provider + Clone,
    token: Address,
    owner: Address,
) -> Result<(U256, String, u8), HeatError> {
    let symbol = erc20::symbol(provider.clone(), token).await?;
    let decimals = erc20::decimals(provider.clone(), token).await?;
    let balance = erc20::balance_of(provider, token, owner).await?;
    Ok((balance, symbol, decimals))
}

/// Well-known ERC-20 token addresses per chain.
/// Returns `None` for unknown symbol/chain combinations.
pub fn well_known_token(chain: EvmChain, symbol: &str) -> Option<(Address, u8)> {
    match (chain, symbol.to_uppercase().as_str()) {
        // USDC
        (EvmChain::Ethereum, "USDC") => {
            Some((address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"), 6))
        }
        (EvmChain::Polygon, "USDC") => {
            Some((address!("3c499c542cef5e3811e1192ce70d8cc03d5c3359"), 6))
        }
        (EvmChain::Arbitrum, "USDC") => {
            Some((address!("af88d065e77c8cc2239327c5edb3a432268e5831"), 6))
        }
        (EvmChain::Optimism, "USDC") => {
            Some((address!("0b2c639c533813f4aa9d7837caf62653d097ff85"), 6))
        }
        (EvmChain::Base, "USDC") => Some((address!("833589fcd6edb6e08f4c7c32d4f71b54bda02913"), 6)),
        // USDT
        (EvmChain::Ethereum, "USDT") => {
            Some((address!("dac17f958d2ee523a2206206994597c13d831ec7"), 6))
        }
        (EvmChain::Polygon, "USDT") => {
            Some((address!("c2132d05d31c914a87c6611c10748aeb04b58e8f"), 6))
        }
        (EvmChain::Arbitrum, "USDT") => {
            Some((address!("fd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9"), 6))
        }
        (EvmChain::Optimism, "USDT") => {
            Some((address!("94b008aa00579c1307b0ef2c499ad98a8ce58e58"), 6))
        }
        // WETH
        (EvmChain::Polygon, "WETH") => {
            Some((address!("7ceb23fd6bc0add59e62ac25578270cff1b9f619"), 18))
        }
        (EvmChain::Arbitrum, "WETH") => {
            Some((address!("82af49447d8a07e3bd95bd0d56f35241523fbab1"), 18))
        }
        (EvmChain::Optimism, "WETH") => {
            Some((address!("4200000000000000000000000000000000000006"), 18))
        }
        (EvmChain::Base, "WETH") => {
            Some((address!("4200000000000000000000000000000000000006"), 18))
        }
        // DAI
        (EvmChain::Ethereum, "DAI") => {
            Some((address!("6b175474e89094c44da98b954eedeac495271d0f"), 18))
        }
        _ => None,
    }
}

/// Parse a token specifier: "native", a well-known symbol, or a 0x address.
pub fn resolve_token(chain: EvmChain, spec: &str) -> Result<TokenSpec, HeatError> {
    let s = spec.trim();
    if s.eq_ignore_ascii_case("native") {
        return Ok(TokenSpec::Native);
    }
    // Try as address
    if s.starts_with("0x") || s.starts_with("0X") {
        let addr: Address = s.parse().map_err(|_| {
            HeatError::validation(
                "invalid_token_address",
                format!("Invalid token address: '{s}'"),
            )
        })?;
        return Ok(TokenSpec::Erc20 {
            address: addr,
            known_symbol: None,
            known_decimals: None,
        });
    }
    // Try as well-known symbol
    if let Some((addr, decimals)) = well_known_token(chain, s) {
        return Ok(TokenSpec::Erc20 {
            address: addr,
            known_symbol: Some(s.to_uppercase()),
            known_decimals: Some(decimals),
        });
    }
    Err(HeatError::validation(
        "unknown_token",
        format!("Unknown token '{s}' on {}", chain.canonical_name()),
    )
    .with_hint("Use a 0x address, 'native', or a known symbol: USDC, USDT, WETH, DAI"))
}

/// Resolved token specification.
#[derive(Debug, Clone)]
pub enum TokenSpec {
    Native,
    Erc20 {
        address: Address,
        known_symbol: Option<String>,
        known_decimals: Option<u8>,
    },
}

/// Parse a comma-separated chain list.
pub fn parse_chains(input: &str) -> Result<Vec<EvmChain>, HeatError> {
    let mut chains = Vec::new();
    for part in input.split(',') {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            chains.push(EvmChain::from_name(trimmed)?);
        }
    }
    if chains.is_empty() {
        return Err(HeatError::validation("no_chains", "No chains specified"));
    }
    Ok(chains)
}

/// Parse a comma-separated token list.
pub fn parse_tokens(input: &str, chain: EvmChain) -> Result<Vec<TokenSpec>, HeatError> {
    let mut tokens = Vec::new();
    for part in input.split(',') {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            tokens.push(resolve_token(chain, trimmed)?);
        }
    }
    if tokens.is_empty() {
        return Err(HeatError::validation("no_tokens", "No tokens specified"));
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_native_token() {
        let spec = resolve_token(EvmChain::Ethereum, "native").unwrap();
        assert!(matches!(spec, TokenSpec::Native));
    }

    #[test]
    fn parse_native_case_insensitive() {
        let spec = resolve_token(EvmChain::Ethereum, "NATIVE").unwrap();
        assert!(matches!(spec, TokenSpec::Native));
    }

    #[test]
    fn parse_usdc_ethereum() {
        let spec = resolve_token(EvmChain::Ethereum, "USDC").unwrap();
        match spec {
            TokenSpec::Erc20 {
                address,
                known_symbol,
                known_decimals,
            } => {
                assert_eq!(
                    format!("{address:#x}"),
                    "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
                );
                assert_eq!(known_symbol.as_deref(), Some("USDC"));
                assert_eq!(known_decimals, Some(6));
            }
            _ => panic!("expected Erc20"),
        }
    }

    #[test]
    fn parse_usdc_lowercase() {
        let spec = resolve_token(EvmChain::Base, "usdc").unwrap();
        assert!(matches!(spec, TokenSpec::Erc20 { .. }));
    }

    #[test]
    fn parse_address_token() {
        let spec = resolve_token(
            EvmChain::Ethereum,
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        )
        .unwrap();
        match spec {
            TokenSpec::Erc20 { known_symbol, .. } => {
                assert!(known_symbol.is_none());
            }
            _ => panic!("expected Erc20"),
        }
    }

    #[test]
    fn unknown_symbol_errors() {
        let err = resolve_token(EvmChain::Ethereum, "SHIB").unwrap_err();
        assert_eq!(err.reason, "unknown_token");
    }

    #[test]
    fn parse_chain_list() {
        let chains = parse_chains("ethereum,base,arbitrum").unwrap();
        assert_eq!(chains.len(), 3);
        assert_eq!(chains[0], EvmChain::Ethereum);
        assert_eq!(chains[1], EvmChain::Base);
        assert_eq!(chains[2], EvmChain::Arbitrum);
    }

    #[test]
    fn parse_chain_list_with_spaces() {
        let chains = parse_chains("eth, poly, arb").unwrap();
        assert_eq!(chains.len(), 3);
    }

    #[test]
    fn parse_empty_chain_list() {
        assert!(parse_chains("").is_err());
    }

    #[test]
    fn parse_token_list() {
        let tokens = parse_tokens("native,USDC", EvmChain::Ethereum).unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0], TokenSpec::Native));
        assert!(matches!(tokens[1], TokenSpec::Erc20 { .. }));
    }

    #[test]
    fn well_known_usdc_all_chains() {
        for chain in EvmChain::all() {
            let result = well_known_token(*chain, "USDC");
            // USDC is available on all 5 chains
            assert!(result.is_some(), "USDC should be known on {chain}");
            let (_, decimals) = result.unwrap();
            assert_eq!(decimals, 6);
        }
    }

    #[test]
    fn well_known_usdt_no_base() {
        // USDT is not yet in our registry for Base
        assert!(well_known_token(EvmChain::Base, "USDT").is_none());
    }
}
