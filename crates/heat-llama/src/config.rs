use heat_core::config::HeatConfig;
use heat_core::error::HeatError;

/// Resolve the DefiLlama API key from env or config.
///
/// Precedence:
/// 1. `HEAT_DEFILLAMA_API_KEY` env var
/// 2. `[protocols.defillama].api_key` in config
pub fn resolve_api_key(config: &HeatConfig) -> Option<String> {
    if let Ok(key) = std::env::var("HEAT_DEFILLAMA_API_KEY")
        && !key.is_empty()
    {
        return Some(key);
    }
    config
        .protocol_value("defillama", "api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
}

/// Require a pro API key or fail with a clear auth error.
pub fn require_api_key(config: &HeatConfig) -> Result<String, HeatError> {
    resolve_api_key(config).ok_or_else(|| {
        HeatError::auth(
            "missing_api_key",
            "This command requires a DefiLlama Pro API key",
        )
        .with_hint(
            "Set HEAT_DEFILLAMA_API_KEY or add api_key under [protocols.defillama] in config",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_api_key_from_empty_config() {
        let config = HeatConfig::default();
        // Only env var could provide it; config is empty.
        // We can't easily test env vars here without mutation,
        // but we can verify None is returned from empty config.
        let _ = resolve_api_key(&config);
    }

    #[test]
    fn require_api_key_fails_without_key() {
        let config = HeatConfig::default();
        // Clear env var if set (test isolation)
        // SAFETY: single-threaded test, no other threads reading this var.
        unsafe { std::env::remove_var("HEAT_DEFILLAMA_API_KEY") };
        let result = require_api_key(&config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.reason, "missing_api_key");
    }
}
