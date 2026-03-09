//! Supported Solana clusters.

use heat_core::error::HeatError;

/// The Solana clusters that Heat supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolanaCluster {
    Mainnet,
    Devnet,
}

impl SolanaCluster {
    /// Canonical short name used in config keys, env vars, etc.
    pub fn canonical_name(&self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Devnet => "devnet",
        }
    }

    /// Parse a cluster from a string, accepting common aliases.
    ///
    /// Accepted names:
    /// - `"mainnet"`, `"main"`, `"mainnet-beta"` → `Mainnet`
    /// - `"devnet"`, `"dev"` → `Devnet`
    pub fn from_name(name: &str) -> Result<Self, HeatError> {
        match name.trim().to_ascii_lowercase().as_str() {
            "mainnet" | "main" | "mainnet-beta" => Ok(Self::Mainnet),
            "devnet" | "dev" => Ok(Self::Devnet),
            other => Err(HeatError::validation(
                "unknown_cluster",
                format!(
                    "Unknown Solana cluster: '{other}'. \
                     Valid options: mainnet, devnet"
                ),
            )
            .with_hint("Use 'mainnet' or 'devnet'")),
        }
    }

    /// Built-in default public RPC endpoint for this cluster.
    ///
    /// These are last-resort fallbacks; production use should configure a
    /// private or dedicated RPC via `HEAT_RPC_SOLANA` / Heat config.
    pub fn default_rpc_url(&self) -> &'static str {
        match self {
            Self::Mainnet => "https://api.mainnet-beta.solana.com",
            Self::Devnet => "https://api.devnet.solana.com",
        }
    }
}

impl std::fmt::Display for SolanaCluster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.canonical_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_names() {
        assert_eq!(SolanaCluster::Mainnet.canonical_name(), "mainnet");
        assert_eq!(SolanaCluster::Devnet.canonical_name(), "devnet");
    }

    #[test]
    fn from_name_canonical() {
        assert_eq!(
            SolanaCluster::from_name("mainnet").unwrap(),
            SolanaCluster::Mainnet
        );
        assert_eq!(
            SolanaCluster::from_name("devnet").unwrap(),
            SolanaCluster::Devnet
        );
    }

    #[test]
    fn from_name_aliases() {
        assert_eq!(
            SolanaCluster::from_name("mainnet-beta").unwrap(),
            SolanaCluster::Mainnet
        );
        assert_eq!(
            SolanaCluster::from_name("main").unwrap(),
            SolanaCluster::Mainnet
        );
        assert_eq!(
            SolanaCluster::from_name("dev").unwrap(),
            SolanaCluster::Devnet
        );
    }

    #[test]
    fn from_name_case_insensitive() {
        assert_eq!(
            SolanaCluster::from_name("MAINNET").unwrap(),
            SolanaCluster::Mainnet
        );
        assert_eq!(
            SolanaCluster::from_name("DevNet").unwrap(),
            SolanaCluster::Devnet
        );
    }

    #[test]
    fn from_name_invalid() {
        let err = SolanaCluster::from_name("testnet").unwrap_err();
        assert_eq!(err.reason, "unknown_cluster");

        let err = SolanaCluster::from_name("").unwrap_err();
        assert_eq!(err.reason, "unknown_cluster");
    }

    #[test]
    fn default_rpc_urls_are_non_empty() {
        assert!(!SolanaCluster::Mainnet.default_rpc_url().is_empty());
        assert!(!SolanaCluster::Devnet.default_rpc_url().is_empty());
    }
}
