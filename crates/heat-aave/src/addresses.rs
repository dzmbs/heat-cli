/// Canonical Aave V3 market addresses per chain.
///
/// Seeded from the official Aave address-book. The `PoolAddressesProvider`
/// is the canonical entrypoint — Pool and helper addresses can be resolved
/// dynamically from it at runtime, but we store well-known addresses for
/// validation and offline tests.
use alloy::primitives::{Address, address};
use heat_core::error::HeatError;
use heat_evm::EvmChain;

/// Aave V3 market descriptor for a single chain.
#[derive(Debug, Clone, Copy)]
pub struct AaveMarket {
    pub chain: EvmChain,
    pub pool_addresses_provider: Address,
    /// Well-known Pool address (for tests/validation only — runtime uses resolver).
    pub pool: Address,
    /// Well-known DataProvider address (for tests/validation only — runtime uses resolver).
    pub protocol_data_provider: Address,
}

/// Look up the Aave V3 market for a given chain.
pub fn market_for_chain(chain: EvmChain) -> Result<&'static AaveMarket, HeatError> {
    MARKETS.iter().find(|m| m.chain == chain).ok_or_else(|| {
        HeatError::validation(
            "unsupported_aave_chain",
            format!("Aave V3 is not supported on {}", chain.canonical_name()),
        )
        .with_hint("Supported chains: ethereum, arbitrum, base")
    })
}

/// All supported Aave V3 markets.
pub fn all_markets() -> &'static [AaveMarket] {
    &MARKETS
}

static MARKETS: [AaveMarket; 3] = [
    // Ethereum
    AaveMarket {
        chain: EvmChain::Ethereum,
        pool_addresses_provider: address!("2f39d218133AFaB8F2B819B1066c7E434Ad94E9e"),
        pool: address!("87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2"),
        protocol_data_provider: address!("0a16f2FCC0D44FaE41cc54e079281D84A363bECD"),
    },
    // Arbitrum
    AaveMarket {
        chain: EvmChain::Arbitrum,
        pool_addresses_provider: address!("a97684ead0e402dC232d5A977953DF7ECBaB3CDb"),
        pool: address!("794a61358D6845594F94dc1DB02A252b5b4814aD"),
        protocol_data_provider: address!("243Aa95cAC2a25651eda86e80bEe66114413c43b"),
    },
    // Base
    AaveMarket {
        chain: EvmChain::Base,
        pool_addresses_provider: address!("e20fCBdBfFC4Dd138cE8b2E6FBb6CB49777ad64D"),
        pool: address!("A238Dd80C259a72e81d7e4664a9801593F98d1c5"),
        protocol_data_provider: address!("0F43731EB8d45A581f4a36DD74F5f358bc90C73A"),
    },
];
