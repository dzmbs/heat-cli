//! RPC URL resolution with Heat precedence rules.
//!
//! Precedence (highest to lowest):
//! 1. Explicit CLI flag (`explicit` argument)
//! 2. Protocol config override (ctx.config.protocols[protocol]["rpc_<chain>"])
//! 3. `HEAT_RPC_ETHEREUM` / `HEAT_RPC_POLYGON` / etc. environment variables
//! 4. Heat config value (ctx.config.protocols["evm"]["rpc_<chain>"])
//! 5. Built-in public RPC defaults (drpc.org public endpoints)

use crate::chains::EvmChain;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;

/// Environment variable name for a given chain's RPC override.
pub(crate) fn env_var_name(chain: EvmChain) -> String {
    format!("HEAT_RPC_{}", chain.canonical_name().to_uppercase())
}

/// Config key under protocols["evm"] for a given chain.
pub(crate) fn config_key(chain: EvmChain) -> String {
    format!("rpc_{}", chain.canonical_name())
}

/// Public fallback RPC endpoints. Last resort only.
fn default_rpc(chain: EvmChain) -> &'static str {
    match chain {
        EvmChain::Ethereum => "https://eth.drpc.org",
        EvmChain::Polygon => "https://polygon.drpc.org",
        EvmChain::Arbitrum => "https://arbitrum.drpc.org",
        EvmChain::Optimism => "https://optimism.drpc.org",
        EvmChain::Base => "https://base.drpc.org",
    }
}

/// Resolve the RPC URL for a chain following Heat precedence rules.
///
/// `explicit` comes from a CLI flag (e.g., `--rpc`).
/// `protocol` is the optional protocol section name to check in config first
/// (e.g., "lifi" or "aave") before falling back to the "evm" section.
pub fn resolve_rpc_url(
    ctx: &Ctx,
    chain: EvmChain,
    explicit: Option<&str>,
    protocol: Option<&str>,
) -> Result<String, HeatError> {
    // 1. Explicit CLI flag
    if let Some(url) = explicit {
        let url = url.trim();
        if !url.is_empty() {
            return Ok(url.to_string());
        }
    }

    let key = config_key(chain);

    // 2. Protocol-specific config override
    if let Some(proto) = protocol
        && let Some(val) = ctx.config.protocol_value(proto, &key)
        && let Some(s) = val.as_str().filter(|s| !s.is_empty())
    {
        return Ok(s.to_string());
    }

    // 3. Environment variable
    let env_var = env_var_name(chain);
    if let Ok(url) = std::env::var(&env_var) {
        let url = url.trim().to_string();
        if !url.is_empty() {
            return Ok(url);
        }
    }

    // 4. Heat config — "evm" section
    if let Some(val) = ctx.config.protocol_value("evm", &key)
        && let Some(s) = val.as_str().filter(|s| !s.is_empty())
    {
        return Ok(s.to_string());
    }

    // 5. Built-in default
    Ok(default_rpc(chain).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use heat_core::config::HeatConfig;
    use heat_core::ctx::Ctx;
    use heat_core::output::OutputFormat;

    fn test_ctx() -> Ctx {
        Ctx::new(
            OutputFormat::Json,
            HeatConfig::default(),
            None,
            None,
            false,
            false,
        )
        .expect("test ctx")
    }

    /// Clear all EVM RPC env vars to ensure test isolation.
    fn clear_all_evm_env() {
        for chain in EvmChain::all() {
            unsafe { std::env::remove_var(env_var_name(*chain)) };
        }
    }

    #[test]
    fn env_var_names_are_correct() {
        assert_eq!(env_var_name(EvmChain::Ethereum), "HEAT_RPC_ETHEREUM");
        assert_eq!(env_var_name(EvmChain::Polygon), "HEAT_RPC_POLYGON");
        assert_eq!(env_var_name(EvmChain::Arbitrum), "HEAT_RPC_ARBITRUM");
        assert_eq!(env_var_name(EvmChain::Optimism), "HEAT_RPC_OPTIMISM");
        assert_eq!(env_var_name(EvmChain::Base), "HEAT_RPC_BASE");
    }

    #[test]
    fn explicit_takes_highest_precedence() {
        let ctx = test_ctx();
        let url = resolve_rpc_url(
            &ctx,
            EvmChain::Ethereum,
            Some("https://my-rpc.example.com"),
            None,
        )
        .unwrap();
        assert_eq!(url, "https://my-rpc.example.com");
    }

    #[test]
    fn falls_back_to_default_when_nothing_set() {
        // SAFETY: test-only env mutation; tests run single-threaded via --test-threads=1
        // or accept the inherent race in parallel test runs.
        clear_all_evm_env();
        let ctx = test_ctx();
        let url = resolve_rpc_url(&ctx, EvmChain::Base, None, None).unwrap();
        assert_eq!(url, "https://base.drpc.org");
    }

    #[test]
    fn env_var_overrides_default() {
        // SAFETY: test-only env mutation
        clear_all_evm_env();
        let ctx = test_ctx();
        unsafe { std::env::set_var("HEAT_RPC_OPTIMISM", "https://custom-optimism.example.com") };
        let url = resolve_rpc_url(&ctx, EvmChain::Optimism, None, None).unwrap();
        clear_all_evm_env();
        assert_eq!(url, "https://custom-optimism.example.com");
    }

    #[test]
    fn explicit_overrides_env_var() {
        // SAFETY: test-only env mutation
        clear_all_evm_env();
        let ctx = test_ctx();
        unsafe { std::env::set_var("HEAT_RPC_ETHEREUM", "https://env-rpc.example.com") };
        let url = resolve_rpc_url(
            &ctx,
            EvmChain::Ethereum,
            Some("https://explicit.example.com"),
            None,
        )
        .unwrap();
        clear_all_evm_env();
        assert_eq!(url, "https://explicit.example.com");
    }

    #[test]
    fn all_chains_have_defaults() {
        // SAFETY: test-only env mutation
        clear_all_evm_env();
        let ctx = test_ctx();
        for chain in EvmChain::all() {
            let url = resolve_rpc_url(&ctx, *chain, None, None).unwrap();
            assert!(
                url.starts_with("https://"),
                "default RPC for {chain} should be HTTPS"
            );
        }
    }

    #[test]
    fn config_key_format() {
        assert_eq!(config_key(EvmChain::Ethereum), "rpc_ethereum");
        assert_eq!(config_key(EvmChain::Polygon), "rpc_polygon");
        assert_eq!(config_key(EvmChain::Arbitrum), "rpc_arbitrum");
        assert_eq!(config_key(EvmChain::Optimism), "rpc_optimism");
        assert_eq!(config_key(EvmChain::Base), "rpc_base");
    }
}
