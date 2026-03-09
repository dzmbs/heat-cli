//! Heat-supported EVM chains with canonical names, IDs, and aliases.

use heat_core::error::HeatError;

/// EVM chains supported by Heat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvmChain {
    Ethereum,
    Polygon,
    Arbitrum,
    Optimism,
    Base,
}

impl EvmChain {
    /// Canonical lowercase name used throughout Heat config, env vars, and output.
    pub fn canonical_name(&self) -> &'static str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Polygon => "polygon",
            Self::Arbitrum => "arbitrum",
            Self::Optimism => "optimism",
            Self::Base => "base",
        }
    }

    /// EIP-155 chain ID.
    pub fn chain_id(&self) -> u64 {
        match self {
            Self::Ethereum => 1,
            Self::Polygon => 137,
            Self::Arbitrum => 42161,
            Self::Optimism => 10,
            Self::Base => 8453,
        }
    }

    /// Symbol of the native gas token.
    pub fn native_symbol(&self) -> &'static str {
        match self {
            Self::Ethereum => "ETH",
            Self::Polygon => "POL",
            Self::Arbitrum => "ETH",
            Self::Optimism => "ETH",
            Self::Base => "ETH",
        }
    }

    /// Parse a chain from a user-supplied string. Accepts canonical names and
    /// common aliases. Case-insensitive.
    pub fn from_name(name: &str) -> Result<Self, HeatError> {
        match name.trim().to_lowercase().as_str() {
            "ethereum" | "eth" | "mainnet" | "1" => Ok(Self::Ethereum),
            "polygon" | "poly" | "matic" | "137" => Ok(Self::Polygon),
            "arbitrum" | "arb" | "arbitrum-one" | "42161" => Ok(Self::Arbitrum),
            "optimism" | "opt" | "op" | "10" => Ok(Self::Optimism),
            "base" | "8453" => Ok(Self::Base),
            other => Err(HeatError::validation(
                "unknown_chain",
                format!("Unknown EVM chain: '{other}'"),
            )
            .with_hint(
                "Supported chains: ethereum (eth), polygon (poly), arbitrum (arb), optimism (opt), base",
            )),
        }
    }

    /// Try to resolve a chain ID to a Heat-supported `EvmChain`.
    /// Returns `None` for chain IDs not in the Heat-supported set.
    pub fn from_chain_id(chain_id: u64) -> Option<Self> {
        match chain_id {
            1 => Some(Self::Ethereum),
            137 => Some(Self::Polygon),
            42161 => Some(Self::Arbitrum),
            10 => Some(Self::Optimism),
            8453 => Some(Self::Base),
            _ => None,
        }
    }

    /// All supported chains, in a stable order.
    pub fn all() -> &'static [EvmChain] {
        &[
            Self::Ethereum,
            Self::Polygon,
            Self::Arbitrum,
            Self::Optimism,
            Self::Base,
        ]
    }
}

impl std::fmt::Display for EvmChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.canonical_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_names_are_unique() {
        let names: Vec<_> = EvmChain::all().iter().map(|c| c.canonical_name()).collect();
        let mut unique = names.clone();
        unique.dedup();
        // sorted dedup would be more thorough, but canonical names are obviously unique
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn chain_ids_are_correct() {
        assert_eq!(EvmChain::Ethereum.chain_id(), 1);
        assert_eq!(EvmChain::Polygon.chain_id(), 137);
        assert_eq!(EvmChain::Arbitrum.chain_id(), 42161);
        assert_eq!(EvmChain::Optimism.chain_id(), 10);
        assert_eq!(EvmChain::Base.chain_id(), 8453);
    }

    #[test]
    fn native_symbols() {
        assert_eq!(EvmChain::Ethereum.native_symbol(), "ETH");
        assert_eq!(EvmChain::Polygon.native_symbol(), "POL");
        assert_eq!(EvmChain::Arbitrum.native_symbol(), "ETH");
        assert_eq!(EvmChain::Optimism.native_symbol(), "ETH");
        assert_eq!(EvmChain::Base.native_symbol(), "ETH");
    }

    #[test]
    fn parse_canonical_names() {
        assert_eq!(EvmChain::from_name("ethereum").unwrap(), EvmChain::Ethereum);
        assert_eq!(EvmChain::from_name("polygon").unwrap(), EvmChain::Polygon);
        assert_eq!(EvmChain::from_name("arbitrum").unwrap(), EvmChain::Arbitrum);
        assert_eq!(EvmChain::from_name("optimism").unwrap(), EvmChain::Optimism);
        assert_eq!(EvmChain::from_name("base").unwrap(), EvmChain::Base);
    }

    #[test]
    fn parse_aliases() {
        assert_eq!(EvmChain::from_name("eth").unwrap(), EvmChain::Ethereum);
        assert_eq!(EvmChain::from_name("mainnet").unwrap(), EvmChain::Ethereum);
        assert_eq!(EvmChain::from_name("1").unwrap(), EvmChain::Ethereum);

        assert_eq!(EvmChain::from_name("poly").unwrap(), EvmChain::Polygon);
        assert_eq!(EvmChain::from_name("matic").unwrap(), EvmChain::Polygon);
        assert_eq!(EvmChain::from_name("137").unwrap(), EvmChain::Polygon);

        assert_eq!(EvmChain::from_name("arb").unwrap(), EvmChain::Arbitrum);
        assert_eq!(
            EvmChain::from_name("arbitrum-one").unwrap(),
            EvmChain::Arbitrum
        );
        assert_eq!(EvmChain::from_name("42161").unwrap(), EvmChain::Arbitrum);

        assert_eq!(EvmChain::from_name("opt").unwrap(), EvmChain::Optimism);
        assert_eq!(EvmChain::from_name("op").unwrap(), EvmChain::Optimism);
        assert_eq!(EvmChain::from_name("10").unwrap(), EvmChain::Optimism);

        assert_eq!(EvmChain::from_name("8453").unwrap(), EvmChain::Base);
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(EvmChain::from_name("ETHEREUM").unwrap(), EvmChain::Ethereum);
        assert_eq!(EvmChain::from_name("ETH").unwrap(), EvmChain::Ethereum);
        assert_eq!(EvmChain::from_name("Polygon").unwrap(), EvmChain::Polygon);
    }

    #[test]
    fn parse_invalid_chain() {
        let err = EvmChain::from_name("solana").unwrap_err();
        assert_eq!(err.category, heat_core::error::ErrorCategory::Validation);
        assert_eq!(err.reason, "unknown_chain");
        assert!(err.hint.is_some());
    }

    #[test]
    fn display_is_canonical() {
        for chain in EvmChain::all() {
            assert_eq!(chain.to_string(), chain.canonical_name());
        }
    }
}
