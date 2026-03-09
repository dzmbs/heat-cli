//! Solana RPC URL resolution with Heat precedence rules.
//!
//! Resolution order (first non-empty wins):
//! 1. Explicit flag value passed by the caller
//! 2. Protocol config: `[protocols.solana] rpc_<cluster>` or `rpc`
//! 3. Cluster-specific env var: `HEAT_RPC_SOLANA_MAINNET` / `HEAT_RPC_SOLANA_DEVNET`
//! 4. Generic Solana env var: `HEAT_RPC_SOLANA`
//! 5. Built-in public default for the cluster

use heat_core::{ctx::Ctx, error::HeatError};

use crate::cluster::SolanaCluster;

/// Resolve the Solana RPC URL to use, applying Heat's precedence rules.
///
/// `explicit` is typically wired from a `--rpc-url` CLI flag.
pub fn resolve_rpc_url(
    ctx: &Ctx,
    cluster: SolanaCluster,
    explicit: Option<&str>,
) -> Result<String, HeatError> {
    // 1. Explicit CLI flag.
    if let Some(url) = explicit.filter(|s| !s.is_empty()) {
        return Ok(url.to_string());
    }

    // 2. Protocol config: [protocols.solana] rpc_mainnet / rpc_devnet / rpc
    let cluster_key = format!("rpc_{}", cluster.canonical_name()); // e.g. "rpc_mainnet"
    if let Some(url) = config_string(ctx, "solana", &cluster_key) {
        return Ok(url);
    }
    if let Some(url) = config_string(ctx, "solana", "rpc") {
        return Ok(url);
    }

    // 3. Cluster-specific env var: HEAT_RPC_SOLANA_MAINNET / HEAT_RPC_SOLANA_DEVNET
    let cluster_env = format!(
        "HEAT_RPC_SOLANA_{}",
        cluster.canonical_name().to_ascii_uppercase()
    );
    if let Ok(url) = std::env::var(&cluster_env) {
        let url = url.trim().to_string();
        if !url.is_empty() {
            return Ok(url);
        }
    }

    // 4. Generic env var.
    if let Ok(url) = std::env::var("HEAT_RPC_SOLANA") {
        let url = url.trim().to_string();
        if !url.is_empty() {
            return Ok(url);
        }
    }

    // 5. Built-in public default.
    Ok(cluster.default_rpc_url().to_string())
}

/// Extract a string value from `[protocols.<protocol>] <key>` in Heat config.
fn config_string(ctx: &Ctx, protocol: &str, key: &str) -> Option<String> {
    ctx.config
        .protocol_value(protocol, key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use heat_core::{
        config::HeatConfig,
        output::{Output, OutputFormat},
    };

    fn make_ctx(config: HeatConfig) -> Ctx {
        Ctx {
            output: Output::new(OutputFormat::Json),
            config,
            account_name: None,
            network: None,
            dry_run: false,
            yes: false,
        }
    }

    /// Clear all Solana RPC env vars so tests do not bleed into each other
    /// regardless of execution order.
    fn clear_all_solana_env() {
        unsafe {
            std::env::remove_var("HEAT_RPC_SOLANA");
            std::env::remove_var("HEAT_RPC_SOLANA_MAINNET");
            std::env::remove_var("HEAT_RPC_SOLANA_DEVNET");
        }
    }

    #[test]
    fn explicit_wins() {
        clear_all_solana_env();
        let ctx = make_ctx(HeatConfig::default());
        let url = resolve_rpc_url(
            &ctx,
            SolanaCluster::Mainnet,
            Some("https://my-rpc.example.com"),
        )
        .unwrap();
        assert_eq!(url, "https://my-rpc.example.com");
    }

    #[test]
    fn falls_back_to_default_when_nothing_set() {
        clear_all_solana_env();
        let ctx = make_ctx(HeatConfig::default());
        let url = resolve_rpc_url(&ctx, SolanaCluster::Mainnet, None).unwrap();
        assert_eq!(url, SolanaCluster::Mainnet.default_rpc_url());
    }

    #[test]
    fn generic_env_var_is_used() {
        clear_all_solana_env();
        unsafe {
            std::env::set_var("HEAT_RPC_SOLANA", "https://env-generic.example.com");
        }
        let ctx = make_ctx(HeatConfig::default());
        let url = resolve_rpc_url(&ctx, SolanaCluster::Mainnet, None).unwrap();
        assert_eq!(url, "https://env-generic.example.com");
        clear_all_solana_env();
    }

    #[test]
    fn cluster_specific_env_var_beats_generic() {
        clear_all_solana_env();
        unsafe {
            std::env::set_var("HEAT_RPC_SOLANA", "https://env-generic.example.com");
            std::env::set_var("HEAT_RPC_SOLANA_MAINNET", "https://env-mainnet.example.com");
        }
        let ctx = make_ctx(HeatConfig::default());
        let url = resolve_rpc_url(&ctx, SolanaCluster::Mainnet, None).unwrap();
        assert_eq!(url, "https://env-mainnet.example.com");
        clear_all_solana_env();
    }
}
