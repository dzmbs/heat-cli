/// On-chain read operations for Aave V3.
///
/// Uses `AaveProtocolDataProvider` for per-reserve queries (parallelized)
/// and `Pool.getUserAccountData` for health summaries. Contract addresses
/// are resolved at runtime via `PoolAddressesProvider` (see `resolver`).
use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use heat_core::error::HeatError;
use heat_evm::amount::format_units;
use heat_evm::EvmChain;

use crate::contracts::{IPool, IPoolDataProvider};
use crate::dto::{
    HealthDto, MarketDto, MarketsListDto, PositionDto, PositionsListDto,
};
use crate::resolver::ResolvedAddresses;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Aave V3 ray = 1e27. Used for rate conversions.
const RAY: u128 = 1_000_000_000_000_000_000_000_000_000;

// ---------------------------------------------------------------------------
// Markets
// ---------------------------------------------------------------------------

/// Fetch all reserve markets for a chain.
pub async fn fetch_markets(
    provider: impl Provider + Clone,
    chain: EvmChain,
    resolved: &ResolvedAddresses,
) -> Result<MarketsListDto, HeatError> {
    let dp = IPoolDataProvider::new(resolved.data_provider, &provider);

    // 1. Get all reserve tokens (symbol + address).
    let tokens = dp.getAllReservesTokens().call().await.map_err(|e| {
        HeatError::network(
            "aave_reserves_tokens",
            format!("Failed to fetch reserve tokens: {e}"),
        )
    })?;

    // 2. For each reserve, query config, data, caps, pause status, and token addresses.
    //    All queries are independent so we run them concurrently.
    let mut handles = Vec::with_capacity(tokens.len());

    for token in &tokens {
        let dp_clone = IPoolDataProvider::new(resolved.data_provider, &provider);
        let asset = token.tokenAddress;
        let symbol = token.symbol.clone();

        handles.push(async move {
            let (config, data, caps, paused, addrs) = tokio::try_join!(
                async {
                    dp_clone
                        .getReserveConfigurationData(asset)
                        .call()
                        .await
                        .map_err(|e| HeatError::network(
                            "aave_reserve_config",
                            format!("Failed to fetch config for {symbol}: {e}"),
                        ))
                },
                async {
                    dp_clone
                        .getReserveData(asset)
                        .call()
                        .await
                        .map_err(|e| HeatError::network(
                            "aave_reserve_data",
                            format!("Failed to fetch data for {symbol}: {e}"),
                        ))
                },
                async {
                    dp_clone
                        .getReserveCaps(asset)
                        .call()
                        .await
                        .map_err(|e| HeatError::network(
                            "aave_reserve_caps",
                            format!("Failed to fetch caps for {symbol}: {e}"),
                        ))
                },
                async {
                    dp_clone
                        .getPaused(asset)
                        .call()
                        .await
                        .map_err(|e| HeatError::network(
                            "aave_reserve_paused",
                            format!("Failed to fetch paused for {symbol}: {e}"),
                        ))
                },
                async {
                    dp_clone
                        .getReserveTokensAddresses(asset)
                        .call()
                        .await
                        .map_err(|e| HeatError::network(
                            "aave_reserve_tokens_addrs",
                            format!("Failed to fetch token addresses for {symbol}: {e}"),
                        ))
                },
            )?;

            let decimals: u8 = config.decimals.try_into().unwrap_or(18);

            let supply_apy = ray_to_apy_pct(data.liquidityRate);
            let variable_borrow_apy = ray_to_apy_pct(data.variableBorrowRate);

            Ok::<MarketDto, HeatError>(MarketDto {
                symbol: symbol.clone(),
                underlying_address: format!("{:#x}", asset),
                decimals,
                a_token_address: format!("{:#x}", addrs.aTokenAddress),
                variable_debt_token_address: format!("{:#x}", addrs.variableDebtTokenAddress),
                collateral_enabled: config.usageAsCollateralEnabled,
                borrowing_enabled: config.borrowingEnabled,
                is_active: config.isActive,
                is_frozen: config.isFrozen,
                is_paused: paused,
                supply_cap: caps.supplyCap.to_string(),
                borrow_cap: caps.borrowCap.to_string(),
                total_supplied: data.totalAToken.to_string(),
                total_stable_debt: data.totalStableDebt.to_string(),
                total_variable_debt: data.totalVariableDebt.to_string(),
                supply_apy,
                variable_borrow_apy,
                ltv_bps: config.ltv.try_into().unwrap_or(0),
                liquidation_threshold_bps: config.liquidationThreshold.try_into().unwrap_or(0),
            })
        });
    }

    let results = futures::future::join_all(handles).await;
    let mut markets = Vec::with_capacity(results.len());
    for r in results {
        markets.push(r?);
    }

    // Sort by symbol for deterministic output.
    markets.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    Ok(MarketsListDto {
        chain: chain.canonical_name().to_owned(),
        markets,
    })
}

// ---------------------------------------------------------------------------
// Positions
// ---------------------------------------------------------------------------

/// Fetch the user's per-reserve positions for a chain.
pub async fn fetch_positions(
    provider: impl Provider + Clone,
    chain: EvmChain,
    resolved: &ResolvedAddresses,
    user: Address,
) -> Result<PositionsListDto, HeatError> {
    let dp = IPoolDataProvider::new(resolved.data_provider, &provider);

    // 1. Get all reserve tokens.
    let tokens = dp.getAllReservesTokens().call().await.map_err(|e| {
        HeatError::network(
            "aave_reserves_tokens",
            format!("Failed to fetch reserve tokens: {e}"),
        )
    })?;

    // 2. Query user data for each reserve in parallel.
    let mut handles = Vec::with_capacity(tokens.len());

    for token in &tokens {
        let dp_clone = IPoolDataProvider::new(resolved.data_provider, &provider);
        let dp_clone2 = IPoolDataProvider::new(resolved.data_provider, &provider);
        let asset = token.tokenAddress;
        let symbol = token.symbol.clone();

        handles.push(async move {
            let (user_data, config) = tokio::try_join!(
                async {
                    dp_clone
                        .getUserReserveData(asset, user)
                        .call()
                        .await
                        .map_err(|e| HeatError::network(
                            "aave_user_reserve",
                            format!("Failed to fetch user data for {symbol}: {e}"),
                        ))
                },
                async {
                    dp_clone2
                        .getReserveConfigurationData(asset)
                        .call()
                        .await
                        .map_err(|e| HeatError::network(
                            "aave_reserve_config",
                            format!("Failed to fetch config for {symbol}: {e}"),
                        ))
                },
            )?;

            // Skip reserves where user has no position at all.
            if user_data.currentATokenBalance.is_zero()
                && user_data.currentStableDebt.is_zero()
                && user_data.currentVariableDebt.is_zero()
            {
                return Ok(None);
            }

            let decimals: u8 = config.decimals.try_into().unwrap_or(18);

            Ok::<Option<PositionDto>, HeatError>(Some(PositionDto {
                symbol,
                underlying_address: format!("{:#x}", asset),
                decimals,
                supplied: user_data.currentATokenBalance.to_string(),
                stable_debt: user_data.currentStableDebt.to_string(),
                variable_debt: user_data.currentVariableDebt.to_string(),
                collateral_enabled: user_data.usageAsCollateralEnabled,
            }))
        });
    }

    let results = futures::future::join_all(handles).await;
    let mut positions = Vec::new();
    for r in results {
        if let Some(pos) = r? {
            positions.push(pos);
        }
    }

    // Sort by symbol for deterministic output.
    positions.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    Ok(PositionsListDto {
        chain: chain.canonical_name().to_owned(),
        account: format!("{:#x}", user),
        positions,
    })
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

/// Fetch the user's account-level health summary.
pub async fn fetch_health(
    provider: impl Provider + Clone,
    chain: EvmChain,
    resolved: &ResolvedAddresses,
    user: Address,
) -> Result<HealthDto, HeatError> {
    let pool = IPool::new(resolved.pool, &provider);

    let data = pool
        .getUserAccountData(user)
        .call()
        .await
        .map_err(|e| {
            HeatError::network(
                "aave_user_account",
                format!("Failed to fetch user account data: {e}"),
            )
        })?;

    // Health factor is WAD-scaled (1e18). Format as human-readable.
    let hf_display = if data.healthFactor == U256::MAX {
        "\u{221e}".to_owned() // ∞
    } else {
        format_units(data.healthFactor, 18)
    };

    Ok(HealthDto {
        chain: chain.canonical_name().to_owned(),
        account: format!("{:#x}", user),
        total_collateral_base: data.totalCollateralBase.to_string(),
        total_debt_base: data.totalDebtBase.to_string(),
        available_borrows_base: data.availableBorrowsBase.to_string(),
        liquidation_threshold_bps: data
            .currentLiquidationThreshold
            .try_into()
            .unwrap_or(0),
        ltv_bps: data.ltv.try_into().unwrap_or(0),
        health_factor: data.healthFactor.to_string(),
        health_factor_display: hf_display,
    })
}

// ---------------------------------------------------------------------------
// Asset resolution
// ---------------------------------------------------------------------------

/// Resolve an asset by symbol or address on the given chain.
///
/// Both symbol and address inputs are validated against the on-chain
/// reserve list. Non-reserve assets are rejected before execution.
pub async fn resolve_asset(
    provider: impl Provider + Clone,
    resolved: &ResolvedAddresses,
    asset_input: &str,
) -> Result<(Address, String, u8), HeatError> {
    let trimmed = asset_input.trim();

    // Always fetch the reserve list for validation.
    let data_provider = IPoolDataProvider::new(resolved.data_provider, &provider);
    let tokens = data_provider
        .getAllReservesTokens()
        .call()
        .await
        .map_err(|e| {
            HeatError::network(
                "aave_reserves_tokens",
                format!("Failed to fetch reserve tokens: {e}"),
            )
        })?;

    // If it looks like an address, validate it against the reserve list.
    if trimmed.starts_with("0x") && trimmed.len() == 42 {
        let addr: Address = trimmed.parse().map_err(|_| {
            HeatError::validation(
                "invalid_address",
                format!("'{trimmed}' is not a valid EVM address"),
            )
        })?;

        let reserve = tokens.iter().find(|t| t.tokenAddress == addr);
        return match reserve {
            Some(t) => {
                let decimals = heat_evm::erc20::decimals(&provider, addr).await?;
                Ok((addr, t.symbol.clone(), decimals))
            }
            None => Err(HeatError::validation(
                "not_aave_reserve",
                format!("Address {trimmed} is not an Aave reserve on this market"),
            )
            .with_hint(
                "Only Aave-listed reserve tokens can be used. Run 'heat aave markets' to see available reserves",
            )),
        };
    }

    // Otherwise, look up by symbol.
    let input_upper = trimmed.to_uppercase();
    let matched: Vec<_> = tokens
        .iter()
        .filter(|t| t.symbol.to_uppercase() == input_upper)
        .collect();

    match matched.len() {
        0 => Err(HeatError::validation(
            "unknown_asset",
            format!("No Aave reserve found with symbol '{trimmed}'"),
        )
        .with_hint("Use the exact symbol (e.g., USDC, WETH, DAI) or pass the token address")),
        1 => {
            let token = matched[0];
            let decimals = heat_evm::erc20::decimals(&provider, token.tokenAddress).await?;
            Ok((token.tokenAddress, token.symbol.clone(), decimals))
        }
        _ => Err(HeatError::validation(
            "ambiguous_asset",
            format!(
                "Multiple reserves match symbol '{trimmed}': {}",
                matched
                    .iter()
                    .map(|t| format!("{} ({:#x})", t.symbol, t.tokenAddress))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
        .with_hint("Pass the token address to disambiguate")),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a ray-scaled interest rate (U256) to an APY percentage string.
/// Simple linear conversion: APY ≈ rate / 1e27 * 100.
/// Ignores compounding for display purposes.
pub(crate) fn ray_to_apy_pct(rate: U256) -> String {
    if rate.is_zero() {
        return "0.00".to_owned();
    }
    // (rate * 10000) / RAY gives basis points * 100.
    let bps_x100 = (rate * U256::from(10000u64)) / U256::from(RAY);
    let bps_x100_u64: u64 = bps_x100.try_into().unwrap_or(0);
    let whole = bps_x100_u64 / 100;
    let frac = bps_x100_u64 % 100;
    format!("{whole}.{frac:02}")
}
