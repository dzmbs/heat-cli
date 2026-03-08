use crate::error::HeatError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Heat config — loaded from ~/.heat/config.toml
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HeatConfig {
    /// Default output mode.
    pub output: Option<String>,
    /// Default account name.
    pub default_account: Option<String>,
    /// Default network.
    pub network: Option<String>,
    /// Per-protocol config sections.
    #[serde(default)]
    pub protocols: HashMap<String, toml::Value>,
}

impl HeatConfig {
    /// Heat home directory: ~/.heat/
    pub fn home_dir() -> Result<PathBuf, HeatError> {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).map_err(|_| {
            HeatError::internal("no_home_dir", "Cannot determine home directory")
        })?;
        let heat_home =
            std::env::var("HEAT_HOME").unwrap_or_else(|_| format!("{home}/.heat"));
        Ok(PathBuf::from(heat_home))
    }

    /// Load config from ~/.heat/config.toml. Returns default if file doesn't exist.
    pub fn load() -> Result<Self, HeatError> {
        let path = Self::home_dir()?.join("config.toml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            HeatError::internal("config_read_failed", format!("Failed to read config: {e}"))
        })?;
        toml::from_str(&content).map_err(|e| {
            HeatError::validation(
                "config_parse_failed",
                format!("Invalid config.toml: {e}"),
            )
        })
    }

    /// Get protocol-specific config value.
    pub fn protocol_value(&self, protocol: &str, key: &str) -> Option<&toml::Value> {
        self.protocols.get(protocol)?.get(key)
    }
}

/// Resolve a value with layered precedence: flag > env > config > default.
pub fn resolve<T: Clone>(
    flag: Option<T>,
    env_var: &str,
    config: Option<T>,
    default: T,
    parse: impl Fn(&str) -> Option<T>,
) -> T {
    if let Some(v) = flag {
        return v;
    }
    if let Ok(env_val) = std::env::var(env_var) {
        if let Some(v) = parse(&env_val) {
            return v;
        }
    }
    config.unwrap_or(default)
}
