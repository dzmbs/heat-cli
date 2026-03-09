use alloy::primitives::U256;

use crate::addresses;
use crate::dto::*;
use heat_evm::EvmChain;

// ---------------------------------------------------------------------------
// Address registry
// ---------------------------------------------------------------------------

#[test]
fn all_supported_chains_have_markets() {
    for chain in [EvmChain::Ethereum, EvmChain::Arbitrum, EvmChain::Base] {
        let market = addresses::market_for_chain(chain);
        assert!(
            market.is_ok(),
            "Missing market for {}",
            chain.canonical_name()
        );
    }
}

#[test]
fn unsupported_chain_returns_error() {
    let err = addresses::market_for_chain(EvmChain::Polygon).unwrap_err();
    assert_eq!(err.reason, "unsupported_aave_chain");
    assert!(err.hint.is_some());
}

#[test]
fn market_addresses_are_nonzero() {
    for market in addresses::all_markets() {
        assert!(!market.pool_addresses_provider.is_zero());
        assert!(!market.pool.is_zero());
        assert!(!market.protocol_data_provider.is_zero());
    }
}

#[test]
fn market_addresses_are_unique_per_chain() {
    for market in addresses::all_markets() {
        let addrs = [
            market.pool_addresses_provider,
            market.pool,
            market.protocol_data_provider,
        ];
        for i in 0..addrs.len() {
            for j in (i + 1)..addrs.len() {
                assert_ne!(
                    addrs[i],
                    addrs[j],
                    "Duplicate address on {} at indices {i} and {j}",
                    market.chain.canonical_name()
                );
            }
        }
    }
}

#[test]
fn ethereum_pool_address_matches_known() {
    let market = addresses::market_for_chain(EvmChain::Ethereum).unwrap();
    assert_eq!(
        format!("{:#x}", market.pool),
        "0x87870bca3f3fd6335c3f4ce8392d69350b4fa4e2"
    );
}

#[test]
fn arbitrum_provider_address_matches_known() {
    let market = addresses::market_for_chain(EvmChain::Arbitrum).unwrap();
    assert_eq!(
        format!("{:#x}", market.pool_addresses_provider),
        "0xa97684ead0e402dc232d5a977953df7ecbab3cdb"
    );
}

#[test]
fn base_pool_address_matches_known() {
    let market = addresses::market_for_chain(EvmChain::Base).unwrap();
    assert_eq!(
        format!("{:#x}", market.pool),
        "0xa238dd80c259a72e81d7e4664a9801593f98d1c5"
    );
}

// ---------------------------------------------------------------------------
// DTO serialization — markets
// ---------------------------------------------------------------------------

#[test]
fn market_dto_serializes_all_fields() {
    let dto = MarketDto {
        symbol: "USDC".to_owned(),
        underlying_address: "0xabc".to_owned(),
        decimals: 6,
        a_token_address: "0xdef".to_owned(),
        variable_debt_token_address: "0x123".to_owned(),
        collateral_enabled: true,
        borrowing_enabled: true,
        is_active: true,
        is_frozen: false,
        is_paused: false,
        supply_cap: "1000000".to_owned(),
        borrow_cap: "500000".to_owned(),
        total_supplied: "999000000000".to_owned(),
        total_stable_debt: "100000000".to_owned(),
        total_variable_debt: "500000000000".to_owned(),
        supply_apy: "3.45".to_owned(),
        variable_borrow_apy: "5.67".to_owned(),
        ltv_bps: 8000,
        liquidation_threshold_bps: 8500,
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["symbol"], "USDC");
    assert_eq!(json["decimals"], 6);
    assert_eq!(json["ltv_bps"], 8000);
    assert_eq!(json["supply_apy"], "3.45");
    assert!(json["is_active"].as_bool().unwrap());
    // Stable debt field present.
    assert_eq!(json["total_stable_debt"], "100000000");
    // No bogus available_liquidity field.
    assert!(json.get("available_liquidity").is_none());
}

#[test]
fn market_dto_no_name_field() {
    let dto = MarketDto {
        symbol: "USDC".to_owned(),
        underlying_address: "0xabc".to_owned(),
        decimals: 6,
        a_token_address: "0xdef".to_owned(),
        variable_debt_token_address: "0x123".to_owned(),
        collateral_enabled: true,
        borrowing_enabled: true,
        is_active: true,
        is_frozen: false,
        is_paused: false,
        supply_cap: "0".to_owned(),
        borrow_cap: "0".to_owned(),
        total_supplied: "0".to_owned(),
        total_stable_debt: "0".to_owned(),
        total_variable_debt: "0".to_owned(),
        supply_apy: "0.00".to_owned(),
        variable_borrow_apy: "0.00".to_owned(),
        ltv_bps: 0,
        liquidation_threshold_bps: 0,
    };
    let json = serde_json::to_value(&dto).unwrap();
    // Removed the unreliable "name" field.
    assert!(json.get("name").is_none());
}

#[test]
fn markets_list_dto_serializes() {
    let dto = MarketsListDto {
        chain: "ethereum".to_owned(),
        markets: vec![],
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["chain"], "ethereum");
    assert!(json["markets"].as_array().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// DTO serialization — positions (with stable + variable debt)
// ---------------------------------------------------------------------------

#[test]
fn position_dto_includes_both_debt_types() {
    let dto = PositionDto {
        symbol: "WETH".to_owned(),
        underlying_address: "0xweth".to_owned(),
        decimals: 18,
        supplied: "1000000000000000000".to_owned(),
        stable_debt: "500000000000000000".to_owned(),
        variable_debt: "250000000000000000".to_owned(),
        collateral_enabled: true,
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["symbol"], "WETH");
    assert_eq!(json["supplied"], "1000000000000000000");
    assert_eq!(json["stable_debt"], "500000000000000000");
    assert_eq!(json["variable_debt"], "250000000000000000");
    assert!(json["collateral_enabled"].as_bool().unwrap());
    // No legacy "borrowed" field.
    assert!(json.get("borrowed").is_none());
}

#[test]
fn position_dto_stable_only_user() {
    // A user who only has stable debt should still show up.
    let dto = PositionDto {
        symbol: "DAI".to_owned(),
        underlying_address: "0xdai".to_owned(),
        decimals: 18,
        supplied: "0".to_owned(),
        stable_debt: "100000000000000000000".to_owned(),
        variable_debt: "0".to_owned(),
        collateral_enabled: false,
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["stable_debt"], "100000000000000000000");
    assert_eq!(json["variable_debt"], "0");
    assert_eq!(json["supplied"], "0");
}

// ---------------------------------------------------------------------------
// DTO serialization — health
// ---------------------------------------------------------------------------

#[test]
fn health_dto_serializes() {
    let dto = HealthDto {
        chain: "arbitrum".to_owned(),
        account: "0xabc".to_owned(),
        total_collateral_base: "100000000".to_owned(),
        total_debt_base: "50000000".to_owned(),
        available_borrows_base: "30000000".to_owned(),
        liquidation_threshold_bps: 8500,
        ltv_bps: 8000,
        health_factor: "1850000000000000000".to_owned(),
        health_factor_display: "1.85".to_owned(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["chain"], "arbitrum");
    assert_eq!(json["health_factor_display"], "1.85");
    assert_eq!(json["ltv_bps"], 8000);
}

// ---------------------------------------------------------------------------
// DTO serialization — supply/withdraw results
// ---------------------------------------------------------------------------

#[test]
fn supply_result_dto_serializes_with_approval() {
    let dto = SupplyResultDto {
        chain: "base".to_owned(),
        account: "0xuser".to_owned(),
        asset_symbol: "USDC".to_owned(),
        asset_address: "0xusdc".to_owned(),
        amount: "1000000".to_owned(),
        amount_display: "1.0 USDC".to_owned(),
        tx_hash: "0xtx".to_owned(),
        approval_tx_hash: Some("0xapproval".to_owned()),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["asset_symbol"], "USDC");
    assert_eq!(json["approval_tx_hash"], "0xapproval");
}

#[test]
fn supply_result_dto_null_approval_when_not_needed() {
    let dto = SupplyResultDto {
        chain: "base".to_owned(),
        account: "0xuser".to_owned(),
        asset_symbol: "USDC".to_owned(),
        asset_address: "0xusdc".to_owned(),
        amount: "1000000".to_owned(),
        amount_display: "1.0 USDC".to_owned(),
        tx_hash: "0xtx".to_owned(),
        approval_tx_hash: None,
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert!(json["approval_tx_hash"].is_null());
}

#[test]
fn withdraw_result_dto_serializes() {
    let dto = WithdrawResultDto {
        chain: "ethereum".to_owned(),
        account: "0xuser".to_owned(),
        asset_symbol: "DAI".to_owned(),
        asset_address: "0xdai".to_owned(),
        amount_requested: "1000000000000000000".to_owned(),
        amount_requested_display: "1.0 DAI".to_owned(),
        tx_hash: "0xtx".to_owned(),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["asset_symbol"], "DAI");
    assert_eq!(json["amount_requested_display"], "1.0 DAI");
}

// ---------------------------------------------------------------------------
// Ray/APY conversion
// ---------------------------------------------------------------------------

#[test]
fn ray_to_apy_zero() {
    assert_eq!(crate::read::ray_to_apy_pct(U256::ZERO), "0.00");
}

#[test]
fn ray_to_apy_three_percent() {
    // 3% = 0.03 * 1e27 = 3e25
    let rate = U256::from(30_000_000_000_000_000_000_000_000u128);
    let result = crate::read::ray_to_apy_pct(rate);
    assert_eq!(result, "3.00");
}

#[test]
fn ray_to_apy_fractional() {
    // 5.67% = 0.0567 * 1e27
    let rate = U256::from(56_700_000_000_000_000_000_000_000u128);
    let result = crate::read::ray_to_apy_pct(rate);
    assert_eq!(result, "5.67");
}

#[test]
fn ray_to_apy_small() {
    // 0.01% = 0.0001 * 1e27 = 1e23
    let rate = U256::from(100_000_000_000_000_000_000_000u128);
    let result = crate::read::ray_to_apy_pct(rate);
    assert_eq!(result, "0.01");
}

// ---------------------------------------------------------------------------
// Chain resolution
// ---------------------------------------------------------------------------

#[test]
fn resolve_chain_from_explicit() {
    use heat_core::config::HeatConfig;
    use heat_core::ctx::Ctx;
    use heat_core::output::OutputFormat;
    let ctx = Ctx::new(
        OutputFormat::Json,
        HeatConfig::default(),
        None,
        None,
        false,
        false,
    )
    .expect("test ctx");
    let chain = crate::cmd::resolve_chain(Some("ethereum"), &ctx).unwrap();
    assert_eq!(chain, EvmChain::Ethereum);
}

#[test]
fn resolve_chain_from_network_fallback() {
    use heat_core::config::HeatConfig;
    use heat_core::ctx::Ctx;
    use heat_core::output::OutputFormat;
    let ctx = Ctx::new(
        OutputFormat::Json,
        HeatConfig::default(),
        None,
        Some("arbitrum".to_owned()),
        false,
        false,
    )
    .expect("test ctx");
    // No explicit --chain, should fall back to ctx.network.
    let chain = crate::cmd::resolve_chain(None, &ctx).unwrap();
    assert_eq!(chain, EvmChain::Arbitrum);
}

#[test]
fn resolve_chain_explicit_overrides_network() {
    use heat_core::config::HeatConfig;
    use heat_core::ctx::Ctx;
    use heat_core::output::OutputFormat;
    let ctx = Ctx::new(
        OutputFormat::Json,
        HeatConfig::default(),
        None,
        Some("arbitrum".to_owned()),
        false,
        false,
    )
    .expect("test ctx");
    // Explicit --chain=base should override ctx.network=arbitrum.
    let chain = crate::cmd::resolve_chain(Some("base"), &ctx).unwrap();
    assert_eq!(chain, EvmChain::Base);
}

#[test]
fn resolve_chain_no_chain_no_network_errors() {
    use heat_core::config::HeatConfig;
    use heat_core::ctx::Ctx;
    use heat_core::output::OutputFormat;
    let ctx = Ctx::new(
        OutputFormat::Json,
        HeatConfig::default(),
        None,
        None,
        false,
        false,
    )
    .expect("test ctx");
    let err = crate::cmd::resolve_chain(None, &ctx).unwrap_err();
    assert_eq!(err.reason, "no_chain");
    assert!(err.hint.is_some());
}

// ---------------------------------------------------------------------------
// Resolver — ResolvedAddresses is used by runtime, not static addresses
// ---------------------------------------------------------------------------

#[test]
fn resolved_addresses_fields_accessible() {
    use alloy::primitives::address;
    let ra = crate::resolver::ResolvedAddresses {
        pool: address!("87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2"),
        data_provider: address!("0a16f2FCC0D44FaE41cc54e079281D84A363bECD"),
    };
    assert!(!ra.pool.is_zero());
    assert!(!ra.data_provider.is_zero());
    assert_ne!(ra.pool, ra.data_provider);
}
