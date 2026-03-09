use crate::config::HeatConfig;
use crate::error::HeatError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The chain family an account belongs to.
/// Used by substrate crates to verify account compatibility before executing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountFamily {
    Evm,
    Solana,
}

impl AccountFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Evm => "evm",
            Self::Solana => "solana",
        }
    }
}

impl std::fmt::Display for AccountFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Account kind — extensible for future signer backends.
///
/// Serializes and deserializes using kebab-case strings:
/// - `"evm-local"` (backward-compatible with the old `AccountType::EvmLocal`)
/// - `"solana-local"`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccountKind {
    EvmLocal,
    SolanaLocal,
}

/// Backward-compatible type alias. Existing call sites that still use `AccountType`
/// will continue to compile. New code should prefer `AccountKind`.
#[deprecated(since = "0.2.0", note = "renamed to AccountKind")]
pub type AccountType = AccountKind;

impl AccountKind {
    /// Returns the chain family this account kind belongs to.
    pub fn family(&self) -> AccountFamily {
        match self {
            Self::EvmLocal => AccountFamily::Evm,
            Self::SolanaLocal => AccountFamily::Solana,
        }
    }

    /// `true` if this account kind belongs to the EVM family.
    pub fn is_evm(&self) -> bool {
        self.family() == AccountFamily::Evm
    }

    /// `true` if this account kind belongs to the Solana family.
    pub fn is_solana(&self) -> bool {
        self.family() == AccountFamily::Solana
    }

    /// Human-readable label used in error messages and display output.
    pub fn label(&self) -> &'static str {
        match self {
            Self::EvmLocal => "evm-local",
            Self::SolanaLocal => "solana-local",
        }
    }
}

/// Account metadata stored in ~/.heat/accounts/<name>.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: AccountKind,
    /// Name of the key in ~/.heat/keys/
    pub key_name: String,
    /// Cached public identity derived from the key.
    /// EVM accounts: `0x`-prefixed 40-char hex address.
    /// Solana accounts: base58-encoded 32-byte public key.
    /// Avoids needing password for read-only queries.
    #[serde(default)]
    pub address: Option<String>,
    /// Default network for this account.
    #[serde(default)]
    pub default_network: Option<String>,
    /// Optional password file path used to decrypt this account automatically.
    #[serde(default)]
    pub password_file: Option<String>,
    /// Optional password environment variable name used to decrypt this account automatically.
    #[serde(default)]
    pub password_env: Option<String>,
    /// Protocol-specific settings.
    #[serde(default)]
    pub protocols: std::collections::HashMap<String, toml::Value>,
}

impl Account {
    /// Returns the chain family this account belongs to.
    pub fn family(&self) -> AccountFamily {
        self.account_type.family()
    }

    /// `true` if this is an EVM-family account.
    pub fn is_evm(&self) -> bool {
        self.account_type.is_evm()
    }

    /// `true` if this is a Solana-family account.
    pub fn is_solana(&self) -> bool {
        self.account_type.is_solana()
    }

    /// Assert that this account belongs to the given family.
    ///
    /// Returns a shared, consistently worded compatibility error when it does not.
    /// Substrate crates should call this rather than writing their own mismatch messages.
    ///
    /// ```
    /// # use heat_core::accounts::AccountFamily;
    /// // In a hypothetical EVM substrate:
    /// // account.require_family(AccountFamily::Evm, "hl")?;
    /// ```
    pub fn require_family(&self, required: AccountFamily, protocol: &str) -> Result<(), HeatError> {
        if self.family() == required {
            return Ok(());
        }
        Err(account_family_mismatch_error(
            &self.name,
            self.account_type.label(),
            required.as_str(),
            protocol,
        ))
    }

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
            HeatError::internal(
                "account_serialize",
                format!("Failed to serialize account: {e}"),
            )
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
            HeatError::internal(
                "account_serialize",
                format!("Failed to serialize account: {e}"),
            )
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
            if path.extension().is_some_and(|ext| ext == "toml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                names.push(stem.to_string());
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

// ── Shared compatibility error helpers ─────────────────────────────────────

/// Build the canonical family-mismatch error used by all substrate crates.
///
/// Having a single constructor here keeps the user-facing wording consistent
/// across EVM, Solana, and future protocols.
pub fn account_family_mismatch_error(
    account_name: &str,
    account_kind: &str,
    required_family: &str,
    protocol: &str,
) -> HeatError {
    HeatError::validation(
        "account_family_mismatch",
        format!(
            "Account '{account_name}' is type '{account_kind}', \
             but protocol '{protocol}' requires a {required_family} account"
        ),
    )
    .with_hint(format!(
        "Create or switch to a {required_family} account: heat accounts use <NAME>"
    ))
}

// ── Account resolution ──────────────────────────────────────────────────────

/// Resolve which account to use.
/// Precedence: --account flag > HEAT_ACCOUNT env > config default > ~/.heat/default-account
pub fn resolve_account_name(flag: Option<&str>, config: &HeatConfig) -> Result<String, HeatError> {
    if let Some(name) = flag {
        return Ok(name.to_string());
    }
    if let Ok(name) = std::env::var("HEAT_ACCOUNT")
        && !name.is_empty()
    {
        return Ok(name);
    }
    if let Some(name) = &config.default_account {
        return Ok(name.clone());
    }
    // Check default-account file
    let default_file = HeatConfig::home_dir()?.join("default-account");
    if default_file.exists() {
        let name = std::fs::read_to_string(&default_file)
            .map_err(|e| {
                HeatError::internal(
                    "default_account",
                    format!("Failed to read default-account: {e}"),
                )
            })?
            .trim()
            .to_string();
        if !name.is_empty() {
            return Ok(name);
        }
    }
    Err(HeatError::auth("no_account", "No account specified")
        .with_hint("Use --account <NAME>, set HEAT_ACCOUNT, or run 'heat accounts use <NAME>'"))
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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── AccountKind helpers ──────────────────────────────────────────────

    #[test]
    fn test_account_kind_family() {
        assert_eq!(AccountKind::EvmLocal.family(), AccountFamily::Evm);
        assert_eq!(AccountKind::SolanaLocal.family(), AccountFamily::Solana);
    }

    #[test]
    fn test_account_kind_is_evm_is_solana() {
        assert!(AccountKind::EvmLocal.is_evm());
        assert!(!AccountKind::EvmLocal.is_solana());

        assert!(AccountKind::SolanaLocal.is_solana());
        assert!(!AccountKind::SolanaLocal.is_evm());
    }

    #[test]
    fn test_account_kind_label() {
        assert_eq!(AccountKind::EvmLocal.label(), "evm-local");
        assert_eq!(AccountKind::SolanaLocal.label(), "solana-local");
    }

    // ── Serde round-trips ────────────────────────────────────────────────

    #[test]
    fn test_account_kind_serde_roundtrip_evm() {
        // TOML can't serialize a bare enum — use JSON for the round-trip test.
        let kind = AccountKind::EvmLocal;
        let serialized = serde_json::to_string(&kind).unwrap();
        assert_eq!(serialized, "\"evm-local\"");
        let deserialized: AccountKind = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, AccountKind::EvmLocal);
    }

    #[test]
    fn test_account_kind_serde_roundtrip_solana() {
        let kind = AccountKind::SolanaLocal;
        let serialized = serde_json::to_string(&kind).unwrap();
        assert_eq!(serialized, "\"solana-local\"");
        let deserialized: AccountKind = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, AccountKind::SolanaLocal);
    }

    /// Existing TOML files on disk use `type = "evm-local"`.
    /// They must still deserialize correctly after the AccountType → AccountKind rename.
    #[test]
    fn test_backward_compat_evm_local_toml() {
        let toml_src = r#"
name = "alice"
type = "evm-local"
key_name = "alice"
address = "0xdeadbeef00000000000000000000000000000000"
"#;
        let account: Account = toml::from_str(toml_src).unwrap();
        assert_eq!(account.account_type, AccountKind::EvmLocal);
        assert!(account.is_evm());
        assert!(!account.is_solana());
        assert_eq!(account.name, "alice");
    }

    #[test]
    fn test_solana_local_toml_deserialize() {
        let toml_src = r#"
name = "bob"
type = "solana-local"
key_name = "bob"
address = "4Nd1mBQtrMJVYVfKf2PX98YVWhMFRSNJBnLTMnCQXt4W"
"#;
        let account: Account = toml::from_str(toml_src).unwrap();
        assert_eq!(account.account_type, AccountKind::SolanaLocal);
        assert!(account.is_solana());
        assert!(!account.is_evm());
    }

    // ── Account.require_family ───────────────────────────────────────────

    #[test]
    fn test_require_family_ok_evm() {
        let account = make_account("alice", AccountKind::EvmLocal);
        assert!(account.require_family(AccountFamily::Evm, "hl").is_ok());
    }

    #[test]
    fn test_require_family_ok_solana() {
        let account = make_account("bob", AccountKind::SolanaLocal);
        assert!(
            account
                .require_family(AccountFamily::Solana, "pacifica")
                .is_ok()
        );
    }

    #[test]
    fn test_require_family_mismatch_evm_account_used_for_solana_protocol() {
        let account = make_account("alice", AccountKind::EvmLocal);
        let err = account
            .require_family(AccountFamily::Solana, "pacifica")
            .unwrap_err();
        assert_eq!(err.reason, "account_family_mismatch");
        assert!(
            err.message.contains("evm-local"),
            "message should mention account kind: {}",
            err.message
        );
        assert!(
            err.message.contains("pacifica"),
            "message should mention protocol: {}",
            err.message
        );
        assert!(
            err.message.contains("solana"),
            "message should mention required family: {}",
            err.message
        );
    }

    #[test]
    fn test_require_family_mismatch_solana_account_used_for_evm_protocol() {
        let account = make_account("bob", AccountKind::SolanaLocal);
        let err = account
            .require_family(AccountFamily::Evm, "hl")
            .unwrap_err();
        assert_eq!(err.reason, "account_family_mismatch");
        assert!(err.message.contains("solana-local"));
        assert!(err.message.contains("hl"));
        assert!(err.message.contains("evm"));
    }

    // ── account_family_mismatch_error standalone ─────────────────────────

    #[test]
    fn test_mismatch_error_hint_mentions_heat_accounts_use() {
        let err = account_family_mismatch_error("alice", "evm-local", "solana", "pacifica");
        let hint = err.hint.expect("mismatch error must have a hint");
        assert!(
            hint.contains("heat accounts use"),
            "hint should reference 'heat accounts use': {hint}"
        );
        assert!(
            hint.contains("solana"),
            "hint should mention required family: {hint}"
        );
    }

    #[test]
    fn test_mismatch_error_category_is_validation() {
        let err = account_family_mismatch_error("x", "evm-local", "solana", "p");
        assert_eq!(err.category, crate::error::ErrorCategory::Validation);
    }

    // ── Account family convenience methods ───────────────────────────────

    #[test]
    fn test_account_is_evm_is_solana() {
        let evm = make_account("a", AccountKind::EvmLocal);
        assert!(evm.is_evm());
        assert!(!evm.is_solana());

        let sol = make_account("b", AccountKind::SolanaLocal);
        assert!(sol.is_solana());
        assert!(!sol.is_evm());
    }

    #[test]
    fn test_account_family_display() {
        assert_eq!(AccountFamily::Evm.to_string(), "evm");
        assert_eq!(AccountFamily::Solana.to_string(), "solana");
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn make_account(name: &str, kind: AccountKind) -> Account {
        Account {
            name: name.to_string(),
            account_type: kind,
            key_name: name.to_string(),
            address: None,
            default_network: None,
            password_file: None,
            password_env: None,
            protocols: Default::default(),
        }
    }
}
