use crate::config::HeatConfig;
use crate::error::HeatError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Account type — extensible for future signer types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccountType {
    EvmLocal,
}

/// Account metadata stored in ~/.heat/accounts/<name>.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: AccountType,
    /// Name of the key in ~/.heat/keys/
    pub key_name: String,
    /// Cached address derived from the key (avoids needing password for read-only queries).
    #[serde(default)]
    pub address: Option<String>,
    /// Default network for this account.
    #[serde(default)]
    pub default_network: Option<String>,
    /// Protocol-specific settings.
    #[serde(default)]
    pub protocols: std::collections::HashMap<String, toml::Value>,
}

impl Account {
    /// Check whether an account with this name exists on disk.
    pub fn exists(name: &str) -> Result<bool, HeatError> {
        Ok(accounts_dir()?.join(format!("{name}.toml")).exists())
    }

    /// Load an account by name from ~/.heat/accounts/<name>.toml
    pub fn load(name: &str) -> Result<Self, HeatError> {
        let path = accounts_dir()?.join(format!("{name}.toml"));
        if !path.exists() {
            return Err(
                HeatError::auth("account_not_found", format!("Account not found: {name}"))
                    .with_hint("Use 'heat accounts list' to see available accounts"),
            );
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            HeatError::internal(
                "account_read_failed",
                format!("Failed to read account {name}: {e}"),
            )
        })?;
        toml::from_str(&content).map_err(|e| {
            HeatError::validation(
                "account_parse_failed",
                format!("Invalid account file {name}.toml: {e}"),
            )
        })
    }

    /// Save this account to disk. Fails if an account with this name already exists.
    pub fn save(&self) -> Result<(), HeatError> {
        let dir = accounts_dir()?;
        crate::fs::ensure_dir(&dir)?;
        let path = dir.join(format!("{}.toml", self.name));
        if path.exists() {
            return Err(HeatError::validation(
                "account_exists",
                format!("Account '{}' already exists", self.name),
            )
            .with_hint("Remove the existing account first, or choose a different name"));
        }
        let content = toml::to_string_pretty(self).map_err(|e| {
            HeatError::internal("account_serialize", format!("Failed to serialize account: {e}"))
        })?;
        crate::fs::atomic_write(&path, content.as_bytes())
    }

    /// Update an existing account on disk (overwrites).
    pub fn save_update(&self) -> Result<(), HeatError> {
        let dir = accounts_dir()?;
        let path = dir.join(format!("{}.toml", self.name));
        if !path.exists() {
            return Err(HeatError::auth(
                "account_not_found",
                format!("Account '{}' not found", self.name),
            ));
        }
        let content = toml::to_string_pretty(self).map_err(|e| {
            HeatError::internal("account_serialize", format!("Failed to serialize account: {e}"))
        })?;
        crate::fs::atomic_write(&path, content.as_bytes())
    }

    /// List all account names.
    pub fn list() -> Result<Vec<String>, HeatError> {
        let dir = accounts_dir()?;
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut names = Vec::new();
        let entries = std::fs::read_dir(&dir).map_err(|e| {
            HeatError::internal("accounts_list", format!("Failed to read accounts dir: {e}"))
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| {
                HeatError::internal("accounts_list", format!("Failed to read entry: {e}"))
            })?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// Remove an account.
    pub fn remove(name: &str) -> Result<(), HeatError> {
        let path = accounts_dir()?.join(format!("{name}.toml"));
        if !path.exists() {
            return Err(HeatError::auth(
                "account_not_found",
                format!("Account not found: {name}"),
            ));
        }
        std::fs::remove_file(&path).map_err(|e| {
            HeatError::internal(
                "account_remove",
                format!("Failed to remove account {name}: {e}"),
            )
        })
    }
}

/// Resolve which account to use.
/// Precedence: --account flag > HEAT_ACCOUNT env > config default > ~/.heat/default-account
pub fn resolve_account_name(
    flag: Option<&str>,
    config: &HeatConfig,
) -> Result<String, HeatError> {
    if let Some(name) = flag {
        return Ok(name.to_string());
    }
    if let Ok(name) = std::env::var("HEAT_ACCOUNT") {
        if !name.is_empty() {
            return Ok(name);
        }
    }
    if let Some(name) = &config.default_account {
        return Ok(name.clone());
    }
    // Check default-account file
    let default_file = HeatConfig::home_dir()?.join("default-account");
    if default_file.exists() {
        let name = std::fs::read_to_string(&default_file)
            .map_err(|e| {
                HeatError::internal("default_account", format!("Failed to read default-account: {e}"))
            })?
            .trim()
            .to_string();
        if !name.is_empty() {
            return Ok(name);
        }
    }
    Err(
        HeatError::auth("no_account", "No account specified")
            .with_hint("Use --account <NAME>, set HEAT_ACCOUNT, or run 'heat accounts use <NAME>'"),
    )
}

/// Set the default account.
pub fn set_default_account(name: &str) -> Result<(), HeatError> {
    // Verify account exists
    let _ = Account::load(name)?;
    let path = HeatConfig::home_dir()?.join("default-account");
    crate::fs::ensure_dir(&HeatConfig::home_dir()?)?;
    crate::fs::atomic_write(&path, name.as_bytes())
}

fn accounts_dir() -> Result<PathBuf, HeatError> {
    Ok(HeatConfig::home_dir()?.join("accounts"))
}
