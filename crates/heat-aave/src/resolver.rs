/// Runtime contract address resolution via PoolAddressesProvider.
///
/// This is the canonical way to obtain Pool and ProtocolDataProvider
/// addresses. The static addresses in `addresses.rs` are for tests
/// and sanity checks only — runtime code must go through this resolver.
use alloy::primitives::Address;
use alloy::providers::Provider;
use heat_core::error::HeatError;

use crate::addresses::AaveMarket;
use crate::contracts::IPoolAddressesProvider;

/// Resolved contract addresses for a single Aave V3 market.
///
/// Obtained by querying `PoolAddressesProvider` on-chain.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedAddresses {
    pub pool: Address,
    pub data_provider: Address,
}

/// Query the on-chain PoolAddressesProvider to resolve Pool and
/// ProtocolDataProvider addresses.
pub async fn resolve(
    provider: impl Provider,
    market: &AaveMarket,
) -> Result<ResolvedAddresses, HeatError> {
    let pap = IPoolAddressesProvider::new(market.pool_addresses_provider, &provider);

    let pool = pap.getPool().call().await.map_err(|e| {
        HeatError::network(
            "aave_resolve_pool",
            format!(
                "Failed to resolve Pool from PoolAddressesProvider ({:#x}): {e}",
                market.pool_addresses_provider
            ),
        )
    })?;

    let data_provider = pap.getPoolDataProvider().call().await.map_err(|e| {
        HeatError::network(
            "aave_resolve_data_provider",
            format!(
                "Failed to resolve DataProvider from PoolAddressesProvider ({:#x}): {e}",
                market.pool_addresses_provider
            ),
        )
    })?;

    if pool.is_zero() {
        return Err(HeatError::protocol(
            "aave_zero_pool",
            "PoolAddressesProvider returned zero address for Pool",
        ));
    }
    if data_provider.is_zero() {
        return Err(HeatError::protocol(
            "aave_zero_data_provider",
            "PoolAddressesProvider returned zero address for DataProvider",
        ));
    }

    Ok(ResolvedAddresses {
        pool,
        data_provider,
    })
}
