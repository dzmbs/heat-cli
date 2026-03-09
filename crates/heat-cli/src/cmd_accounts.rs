use clap::{Args, Subcommand};
use heat_core::accounts::{self, Account, AccountKind};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::keystore;
use heat_core::output::OutputFormat;
use rand::RngCore;
use serde::Serialize;

#[derive(Args)]
pub struct AccountsCmd {
    #[command(subcommand)]
    pub command: AccountsSubcommand,
}

#[derive(Subcommand)]
pub enum AccountsSubcommand {
    /// List all accounts
    List,
    /// Show account details
    Get {
        /// Account name
        name: String,
    },
    /// Create a new local account
    Create {
        /// Account name
        name: String,
        /// Account type: evm-local or solana-local
        #[arg(long, value_name = "TYPE", default_value = "evm-local")]
        account_type: String,
        /// Generate a fresh private key locally instead of supplying one
        #[arg(long)]
        generate: bool,
        /// Private key value (WARNING: visible in process list and shell history — prefer --key-file)
        #[arg(long)]
        key: Option<String>,
        /// Path to file containing the private key (recommended over --key)
        #[arg(long)]
        key_file: Option<String>,
        /// Read password from file (file must already exist)
        #[arg(long)]
        password_file: Option<String>,
        /// Read password from this environment variable
        #[arg(long)]
        password_env: Option<String>,
        /// Save password to this file (recommended; also enables generated-password flows)
        #[arg(long)]
        persist_password: Option<String>,
    },
    /// Import an existing keystore file
    Import {
        /// Account name
        name: String,
        /// Account type: evm-local or solana-local
        #[arg(long, value_name = "TYPE", default_value = "evm-local")]
        account_type: String,
        /// Path to keystore file (V3 JSON for EVM, JSON array for Solana)
        #[arg(long)]
        keystore: String,
        /// Read password from file (used for Solana re-encryption or to remember EVM keystore password)
        #[arg(long)]
        password_file: Option<String>,
        /// Read password from this environment variable (used for Solana re-encryption or to remember EVM keystore password)
        #[arg(long)]
        password_env: Option<String>,
        /// Save password to this file after import (Solana re-encryption, or copy existing EVM keystore password)
        #[arg(long)]
        persist_password: Option<String>,
    },
    /// Set the default account
    Use {
        /// Account name
        name: String,
    },
    /// Remove an account (key file is preserved)
    Remove {
        /// Account name
        name: String,
    },
}

#[derive(Serialize)]
struct AccountInfo {
    name: String,
    #[serde(rename = "type")]
    account_type: String,
    key_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password_env: Option<String>,
}

impl From<&Account> for AccountInfo {
    fn from(a: &Account) -> Self {
        Self {
            name: a.name.clone(),
            account_type: a.account_type.label().to_string(),
            key_name: a.key_name.clone(),
            address: a.address.clone(),
            default_network: a.default_network.clone(),
            password_file: a.password_file.clone(),
            password_env: a.password_env.clone(),
        }
    }
}

pub fn run(cmd: AccountsCmd, ctx: &Ctx) -> Result<(), HeatError> {
    match cmd.command {
        AccountsSubcommand::List => list(ctx),
        AccountsSubcommand::Get { name } => get(&name, ctx),
        AccountsSubcommand::Create {
            name,
            account_type,
            generate,
            key,
            key_file,
            password_file,
            password_env,
            persist_password,
        } => create(
            &name,
            &account_type,
            generate,
            key.as_deref(),
            key_file.as_deref(),
            password_file.as_deref(),
            password_env.as_deref(),
            persist_password.as_deref(),
            ctx,
        ),
        AccountsSubcommand::Import {
            name,
            account_type,
            keystore,
            password_file,
            password_env,
            persist_password,
        } => import(
            &name,
            &account_type,
            &keystore,
            password_file.as_deref(),
            password_env.as_deref(),
            persist_password.as_deref(),
            ctx,
        ),
        AccountsSubcommand::Use { name } => use_account(&name, ctx),
        AccountsSubcommand::Remove { name } => remove(&name, ctx),
    }
}

fn list(ctx: &Ctx) -> Result<(), HeatError> {
    let names = Account::list()?;
    if names.is_empty() {
        if ctx.output.format == OutputFormat::Json {
            ctx.output
                .write_data(&Vec::<AccountInfo>::new(), None)
                .map_err(io_err)?;
        } else {
            ctx.output
                .diagnostic("No accounts. Use 'heat accounts create' to create one.");
        }
        return Ok(());
    }

    // Load default account name for marking
    let default = heat_core::config::HeatConfig::home_dir()
        .ok()
        .map(|h| h.join("default-account"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let mut infos = Vec::new();
    for name in &names {
        if let Ok(account) = Account::load(name) {
            infos.push(AccountInfo::from(&account));
        }
    }

    if ctx.output.format == OutputFormat::Json {
        ctx.output.write_data(&infos, None).map_err(io_err)?;
    } else {
        for info in &infos {
            let marker = if info.name == default { " *" } else { "" };
            println!("{:<16} {}{marker}", info.name, info.account_type);
        }
    }
    Ok(())
}

fn get(name: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let account = Account::load(name)?;
    let info = AccountInfo::from(&account);
    if ctx.output.format == OutputFormat::Json {
        ctx.output.write_data(&info, None).map_err(io_err)?;
    } else {
        println!("Name:    {}", info.name);
        println!("Type:    {}", info.account_type);
        println!("Key:     {}", info.key_name);
        if let Some(addr) = &info.address {
            println!("Address: {addr}");
        }
        if let Some(net) = &info.default_network {
            println!("Network: {net}");
        }
        if let Some(path) = &info.password_file {
            println!("Password file: {path}");
        }
        if let Some(var) = &info.password_env {
            println!("Password env:  {var}");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create(
    name: &str,
    account_type_str: &str,
    generate: bool,
    key_direct: Option<&str>,
    key_file: Option<&str>,
    password_file: Option<&str>,
    password_env: Option<&str>,
    persist_password: Option<&str>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    validate_password_flags(password_file, password_env)?;
    require_recovery_strategy(password_file, password_env, persist_password)?;

    let supplied_key = match (generate, key_direct, key_file) {
        (true, None, None) => None,
        (true, Some(_), _) | (true, _, Some(_)) => {
            return Err(HeatError::validation(
                "key_conflict",
                "Cannot combine --generate with --key or --key-file",
            ));
        }
        (false, Some(k), None) => Some(k.to_string()),
        (false, None, Some(path)) => Some(
            std::fs::read_to_string(path)
                .map_err(|e| {
                    HeatError::validation("key_file_read", format!("Failed to read key file: {e}"))
                })?
                .trim()
                .to_string(),
        ),
        (false, Some(_), Some(_)) => {
            return Err(HeatError::validation(
                "key_conflict",
                "Cannot specify both --key and --key-file",
            ));
        }
        (false, None, None) => {
            return Err(HeatError::validation(
                "no_key",
                "Provide --generate, --key, or --key-file",
            )
            .with_hint(
                "Use --generate for a new wallet, or --key-file to import local key material",
            ));
        }
    };

    preflight_name(name)?;

    let account_kind = parse_account_kind(account_type_str)?;

    let password = resolve_or_generate_password(password_file, password_env, persist_password)?;
    if let Some(path) = persist_password {
        persist_password_file(path, &password)?;
    }

    let (key_bytes, address, generated_key) = match account_kind {
        AccountKind::EvmLocal => {
            let bytes = match supplied_key.as_deref() {
                Some(key_input) => {
                    let hex_str = key_input.strip_prefix("0x").unwrap_or(key_input);
                    let bytes = hex::decode(hex_str).map_err(|_| {
                        HeatError::validation("invalid_key", "Private key must be valid hex")
                    })?;
                    if bytes.len() != 32 {
                        return Err(HeatError::validation(
                            "invalid_key_length",
                            "EVM private key must be 32 bytes (64 hex chars)",
                        ));
                    }
                    bytes
                }
                None => generate_evm_private_key()?,
            };
            let addr = keystore::derive_evm_address(&bytes)?;
            (bytes, addr, supplied_key.is_none())
        }
        AccountKind::SolanaLocal => {
            let seed = match supplied_key.as_deref() {
                Some(key_input) => {
                    // Accept base58-encoded 32-byte seed or 64-byte keypair
                    let bytes = bs58::decode(key_input).into_vec().map_err(|_| {
                        HeatError::validation("invalid_key", "Solana key must be base58-encoded")
                    })?;
                    match bytes.len() {
                        32 => bytes,
                        64 => {
                            let seed = bytes[..32].to_vec();
                            // Validate that the pubkey half matches the derived pubkey
                            let expected = derive_solana_pubkey(&seed)?;
                            let actual = bs58::encode(&bytes[32..]).into_string();
                            if actual != expected {
                                return Err(HeatError::validation(
                                    "keypair_mismatch",
                                    "Public key half of keypair does not match derived public key",
                                )
                                .with_hint(
                                    "The last 32 bytes of a 64-byte keypair must be the Ed25519 public key",
                                ));
                            }
                            seed
                        }
                        _ => {
                            return Err(HeatError::validation(
                                "invalid_key_length",
                                "Solana key must be 32 bytes (seed) or 64 bytes (keypair)",
                            ));
                        }
                    }
                }
                None => generate_solana_seed(),
            };
            let addr = derive_solana_pubkey(&seed)?;
            (seed, addr, supplied_key.is_none())
        }
    };

    // Write key first, then account metadata
    keystore::save_key(name, &key_bytes, password.as_bytes(), Some(&address))?;

    let account = Account {
        name: name.to_string(),
        account_type: account_kind,
        key_name: name.to_string(),
        address: Some(address.clone()),
        default_network: None,
        password_file: persist_password
            .map(str::to_string)
            .or_else(|| password_file.map(str::to_string)),
        password_env: password_env.map(str::to_string),
        protocols: Default::default(),
    };
    account.save()?;

    let generated_password = password_file.is_none() && password_env.is_none();
    let recovery = describe_recovery(
        password_file,
        password_env,
        persist_password,
        generated_password,
    );
    if ctx.output.format == OutputFormat::Json {
        let info = AccountInfo::from(&account);
        let result = serde_json::json!({
            "name": info.name,
            "type": info.account_type,
            "key_name": info.key_name,
            "address": info.address,
            "password_file": info.password_file,
            "password_env": info.password_env,
            "recovery": recovery,
            "generated_key": generated_key,
            "generated_password": generated_password,
        });
        ctx.output.write_data(&result, None).map_err(io_err)?;
    } else {
        ctx.output.diagnostic(&format!("Account '{name}' created."));
        ctx.output.diagnostic(&format!("Address:  {address}"));
        if generated_key {
            ctx.output.diagnostic("Key:      generated locally");
        }
        if generated_password {
            ctx.output.diagnostic("Password: generated locally");
        }
        ctx.output.diagnostic(&format!("Recovery: {recovery}"));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn import(
    name: &str,
    account_type_str: &str,
    keystore_path: &str,
    password_file: Option<&str>,
    password_env: Option<&str>,
    persist_password: Option<&str>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    validate_password_flags(password_file, password_env)?;
    preflight_name(name)?;

    let account_kind = parse_account_kind(account_type_str)?;

    let content = std::fs::read_to_string(keystore_path).map_err(|e| {
        HeatError::validation(
            "keystore_read",
            format!("Failed to read keystore file: {e}"),
        )
    })?;

    let (address, account_kind, recovery) = match account_kind {
        AccountKind::EvmLocal => {
            // EVM import copies an already-encrypted V3 keystore.
            // Password flags are optional here and only control how Heat will find
            // the existing keystore password later for signing.
            let parsed: heat_core::keystore::KeystoreFile = serde_json::from_str(&content)
                .map_err(|e| {
                    HeatError::validation(
                        "keystore_parse",
                        format!("Invalid V3 keystore file: {e}"),
                    )
                })?;
            let address = parsed
                .address
                .as_deref()
                .map(keystore::normalize_keystore_address)
                .transpose()?;

            let keys_dir = heat_core::config::HeatConfig::home_dir()?.join("keys");
            heat_core::fs::ensure_dir(&keys_dir)?;
            let dest = keys_dir.join(format!("{name}.json"));
            heat_core::fs::atomic_write_secure(&dest, content.as_bytes())?;

            if let Some(path) = persist_password {
                let password = keystore::resolve_password(password_file, password_env)?.ok_or_else(|| {
                    HeatError::auth(
                        "no_password",
                        "Imported EVM keystore password is required before it can be persisted",
                    )
                    .with_hint("Use --password-file or --password-env together with --persist-password")
                })?;
                persist_password_file(path, &password)?;
            }

            let recovery = if password_file.is_some()
                || password_env.is_some()
                || persist_password.is_some()
            {
                let generated_password = false;
                Some(describe_recovery(
                    password_file,
                    password_env,
                    persist_password,
                    generated_password,
                ))
            } else {
                None
            };

            (address, AccountKind::EvmLocal, recovery)
        }
        AccountKind::SolanaLocal => {
            // Solana import re-encrypts with a Heat password — enforce recoverability.
            require_recovery_strategy(password_file, password_env, persist_password)?;

            let bytes: Vec<u8> = serde_json::from_str(&content).map_err(|e| {
                HeatError::validation(
                    "keystore_parse",
                    format!("Invalid Solana keypair file (expected JSON byte array): {e}"),
                )
            })?;
            if bytes.len() != 64 {
                return Err(HeatError::validation(
                    "invalid_key_length",
                    format!("Solana keypair must be 64 bytes, got {}", bytes.len()),
                ));
            }
            let seed = &bytes[..32];
            let address = derive_solana_pubkey(seed)?;

            let actual_pubkey = bs58::encode(&bytes[32..]).into_string();
            if actual_pubkey != address {
                return Err(HeatError::validation(
                    "keypair_mismatch",
                    "Public key half of keypair does not match derived public key",
                )
                .with_hint(
                    "The last 32 bytes of a 64-byte keypair must be the Ed25519 public key",
                ));
            }

            let password =
                resolve_or_generate_password(password_file, password_env, persist_password)?;
            if let Some(path) = persist_password {
                persist_password_file(path, &password)?;
            }
            keystore::save_key(name, seed, password.as_bytes(), Some(&address))?;

            let generated_password = password_file.is_none() && password_env.is_none();
            let recovery = describe_recovery(
                password_file,
                password_env,
                persist_password,
                generated_password,
            );

            (Some(address), AccountKind::SolanaLocal, Some(recovery))
        }
    };

    let account = Account {
        name: name.to_string(),
        account_type: account_kind,
        key_name: name.to_string(),
        address,
        default_network: None,
        password_file: persist_password
            .map(str::to_string)
            .or_else(|| password_file.map(str::to_string)),
        password_env: password_env.map(str::to_string),
        protocols: Default::default(),
    };
    account.save()?;

    if ctx.output.format == OutputFormat::Json {
        let info = AccountInfo::from(&account);
        if let Some(recovery) = recovery {
            let result = serde_json::json!({
                "name": info.name,
                "type": info.account_type,
                "key_name": info.key_name,
                "address": info.address,
                "password_file": info.password_file,
                "password_env": info.password_env,
                "recovery": recovery,
            });
            ctx.output.write_data(&result, None).map_err(io_err)?;
        } else {
            ctx.output.write_data(&info, None).map_err(io_err)?;
        }
    } else {
        ctx.output
            .diagnostic(&format!("Account '{name}' imported from {keystore_path}."));
        if let Some(recovery) = recovery {
            ctx.output.diagnostic(&format!("Recovery: {recovery}"));
        }
    }
    Ok(())
}

fn use_account(name: &str, ctx: &Ctx) -> Result<(), HeatError> {
    accounts::set_default_account(name)?;
    if ctx.output.format == OutputFormat::Json {
        ctx.output
            .write_data(&serde_json::json!({"default_account": name}), None)
            .map_err(io_err)?;
    } else {
        ctx.output
            .diagnostic(&format!("Default account set to '{name}'."));
    }
    Ok(())
}

fn remove(name: &str, ctx: &Ctx) -> Result<(), HeatError> {
    Account::remove(name)?;

    // Clear stale default-account pointer if this was the default
    if let Ok(home) = heat_core::config::HeatConfig::home_dir() {
        let default_file = home.join("default-account");
        if let Ok(current) = std::fs::read_to_string(&default_file)
            && current.trim() == name
        {
            let _ = std::fs::remove_file(&default_file);
        }
    }

    if ctx.output.format == OutputFormat::Json {
        ctx.output
            .write_data(&serde_json::json!({"removed": name}), None)
            .map_err(io_err)?;
    } else {
        ctx.output.diagnostic(&format!(
            "Account '{name}' removed. Key file preserved in ~/.heat/keys/."
        ));
    }
    Ok(())
}

fn parse_account_kind(s: &str) -> Result<AccountKind, HeatError> {
    match s {
        "evm-local" | "evm" => Ok(AccountKind::EvmLocal),
        "solana-local" | "solana" => Ok(AccountKind::SolanaLocal),
        _ => Err(HeatError::validation(
            "invalid_account_type",
            format!("Unknown account type: '{s}'"),
        )
        .with_hint("Valid types: evm-local, solana-local")),
    }
}

/// Derive a Solana public key (base58) from a 32-byte Ed25519 seed.
fn derive_solana_pubkey(seed: &[u8]) -> Result<String, HeatError> {
    use ed25519_dalek::SigningKey;
    let seed_array: [u8; 32] = seed
        .try_into()
        .map_err(|_| HeatError::validation("invalid_key_length", "Solana seed must be 32 bytes"))?;
    let signing_key = SigningKey::from_bytes(&seed_array);
    let pubkey = signing_key.verifying_key();
    Ok(bs58::encode(pubkey.as_bytes()).into_string())
}

/// Preflight check: ensure neither account nor key already exists.
/// Must be called before any writes to avoid orphan state.
fn preflight_name(name: &str) -> Result<(), HeatError> {
    if Account::exists(name)? {
        return Err(HeatError::validation(
            "account_exists",
            format!("Account '{name}' already exists"),
        )
        .with_hint("Remove the existing account first, or choose a different name"));
    }
    if keystore::key_exists(name)? {
        return Err(
            HeatError::validation("key_exists", format!("Key '{name}' already exists"))
                .with_hint("Remove the existing key first, or choose a different name"),
        );
    }
    Ok(())
}

/// Enforce that account creation/import has a durable password recovery strategy.
/// HEAT_PASSWORD alone is not sufficient — it lives only in process memory.
fn require_recovery_strategy(
    password_file: Option<&str>,
    password_env: Option<&str>,
    persist_password: Option<&str>,
) -> Result<(), HeatError> {
    if password_file.is_some() || password_env.is_some() || persist_password.is_some() {
        return Ok(());
    }
    Err(HeatError::validation(
        "no_recovery_strategy",
        "Account creation requires a password recovery strategy",
    )
    .with_hint("Use --password-file <PATH>, --password-env <VAR>, or --persist-password <PATH>"))
}

/// Validate password-related flag combinations.
fn validate_password_flags(
    password_file: Option<&str>,
    password_env: Option<&str>,
) -> Result<(), HeatError> {
    let mut sources = 0;
    if password_file.is_some() {
        sources += 1;
    }
    if password_env.is_some() {
        sources += 1;
    }
    if sources > 1 {
        return Err(HeatError::validation(
            "password_source_conflict",
            "Cannot specify both --password-file and --password-env",
        ));
    }

    Ok(())
}

/// Resolve encryption password from explicit sources, or generate one when persistence was requested.
fn resolve_or_generate_password(
    password_file: Option<&str>,
    password_env: Option<&str>,
    persist_password: Option<&str>,
) -> Result<String, HeatError> {
    if let Some(password) = keystore::resolve_password(password_file, password_env)? {
        return Ok(password);
    }

    if persist_password.is_some() {
        return Ok(generate_password());
    }

    Err(HeatError::auth("no_password", "Password required for key encryption").with_hint(
        "Use --password-file, --password-env, set HEAT_PASSWORD, or provide --persist-password to generate one automatically",
    ))
}

/// Persist password to a file with secure permissions (chmod 600).
fn persist_password_file(path: &str, password: &str) -> Result<(), HeatError> {
    let p = std::path::Path::new(path);
    if let Some(parent) = p.parent()
        && !parent.as_os_str().is_empty()
    {
        heat_core::fs::ensure_dir(parent)?;
    }
    heat_core::fs::atomic_write_secure(p, password.as_bytes())
}

/// Describe which recovery strategy was used (for output).
fn describe_recovery(
    password_file: Option<&str>,
    password_env: Option<&str>,
    persist_password: Option<&str>,
    generated_password: bool,
) -> String {
    if let Some(path) = persist_password {
        if generated_password {
            return format!("generated password persisted to {path}");
        }
        return format!("password persisted to {path}");
    }
    if let Some(path) = password_file {
        return format!("password-file {path}");
    }
    if let Some(var) = password_env {
        return format!("password-env ${var}");
    }
    "unknown".to_string()
}

fn generate_password() -> String {
    let mut bytes = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn generate_evm_private_key() -> Result<Vec<u8>, HeatError> {
    loop {
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        if keystore::derive_evm_address(&bytes).is_ok() {
            return Ok(bytes.to_vec());
        }
    }
}

fn generate_solana_seed() -> Vec<u8> {
    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    seed.to_vec()
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Failed to write output: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_conflicting_password_sources() {
        let err = validate_password_flags(Some("a"), Some("ENV")).unwrap_err();
        assert_eq!(err.reason, "password_source_conflict");
    }

    #[test]
    fn generates_password_when_persisting_without_explicit_source() {
        let pw = resolve_or_generate_password(None, None, Some("/tmp/pw")).unwrap();
        assert!(!pw.is_empty());
        assert!(pw.len() >= 32);
    }

    #[test]
    fn generated_evm_key_is_valid() {
        let key = generate_evm_private_key().unwrap();
        assert_eq!(key.len(), 32);
        let addr = keystore::derive_evm_address(&key).unwrap();
        assert!(addr.starts_with("0x"));
    }

    #[test]
    fn generated_solana_seed_is_valid() {
        let seed = generate_solana_seed();
        assert_eq!(seed.len(), 32);
        let addr = derive_solana_pubkey(&seed).unwrap();
        assert!(!addr.is_empty());
    }
}
