use clap::{Args, Subcommand};
use heat_core::accounts::{self, Account, AccountKind};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::keystore;
use heat_core::output::OutputFormat;
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
    /// Create a new account from a private key
    Create {
        /// Account name
        name: String,
        /// Account type: evm-local or solana-local
        #[arg(long, value_name = "TYPE", default_value = "evm-local")]
        account_type: String,
        /// Private key value (WARNING: visible in process list and shell history — prefer --key-file)
        #[arg(long)]
        key: Option<String>,
        /// Path to file containing the private key (recommended over --key)
        #[arg(long)]
        key_file: Option<String>,
        /// Password file for key encryption
        #[arg(long)]
        password_file: Option<String>,
        /// Environment variable containing password
        #[arg(long)]
        password_env: Option<String>,
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
        /// Password file for key encryption (Solana imports only)
        #[arg(long)]
        password_file: Option<String>,
        /// Environment variable containing password (Solana imports only)
        #[arg(long)]
        password_env: Option<String>,
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
}

impl From<&Account> for AccountInfo {
    fn from(a: &Account) -> Self {
        Self {
            name: a.name.clone(),
            account_type: a.account_type.label().to_string(),
            key_name: a.key_name.clone(),
            address: a.address.clone(),
            default_network: a.default_network.clone(),
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
            key,
            key_file,
            password_file,
            password_env,
        } => create(
            &name,
            &account_type,
            key.as_deref(),
            key_file.as_deref(),
            password_file.as_deref(),
            password_env.as_deref(),
            ctx,
        ),
        AccountsSubcommand::Import {
            name,
            account_type,
            keystore,
            password_file,
            password_env,
        } => import(
            &name,
            &account_type,
            &keystore,
            password_file.as_deref(),
            password_env.as_deref(),
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
    }
    Ok(())
}

fn create(
    name: &str,
    account_type_str: &str,
    key_direct: Option<&str>,
    key_file: Option<&str>,
    password_file: Option<&str>,
    password_env: Option<&str>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    let key_input = match (key_direct, key_file) {
        (Some(k), None) => k.to_string(),
        (None, Some(path)) => std::fs::read_to_string(path)
            .map_err(|e| {
                HeatError::validation("key_file_read", format!("Failed to read key file: {e}"))
            })?
            .trim()
            .to_string(),
        (Some(_), Some(_)) => {
            return Err(HeatError::validation(
                "key_conflict",
                "Cannot specify both --key and --key-file",
            ));
        }
        (None, None) => {
            return Err(HeatError::validation(
                "no_key",
                "Either --key or --key-file must be provided",
            )
            .with_hint("Use --key-file to avoid exposing the key in process list"));
        }
    };
    let key_input = key_input.as_str();

    preflight_name(name)?;

    let account_kind = parse_account_kind(account_type_str)?;

    let password = keystore::resolve_password(password_file, password_env)?.ok_or_else(|| {
        HeatError::auth("no_password", "Password required for key encryption")
            .with_hint("Use --password-file, --password-env, or set HEAT_PASSWORD")
    })?;

    let (key_bytes, address) = match account_kind {
        AccountKind::EvmLocal => {
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
            let addr = keystore::derive_evm_address(&bytes)?;
            (bytes, addr)
        }
        AccountKind::SolanaLocal => {
            // Accept base58-encoded 32-byte seed or 64-byte keypair
            let bytes = bs58::decode(key_input).into_vec().map_err(|_| {
                HeatError::validation("invalid_key", "Solana key must be base58-encoded")
            })?;
            let seed = match bytes.len() {
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
            };
            let addr = derive_solana_pubkey(&seed)?;
            (seed, addr)
        }
    };

    // Write key first, then account metadata
    keystore::save_key(name, &key_bytes, password.as_bytes(), Some(&address))?;

    let account = Account {
        name: name.to_string(),
        account_type: account_kind,
        key_name: name.to_string(),
        address: Some(address),
        default_network: None,
        protocols: Default::default(),
    };
    account.save()?;

    if ctx.output.format == OutputFormat::Json {
        let info = AccountInfo::from(&account);
        ctx.output.write_data(&info, None).map_err(io_err)?;
    } else {
        ctx.output.diagnostic(&format!("Account '{name}' created."));
    }
    Ok(())
}

fn import(
    name: &str,
    account_type_str: &str,
    keystore_path: &str,
    password_file: Option<&str>,
    password_env: Option<&str>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    preflight_name(name)?;

    let account_kind = parse_account_kind(account_type_str)?;

    let content = std::fs::read_to_string(keystore_path).map_err(|e| {
        HeatError::validation(
            "keystore_read",
            format!("Failed to read keystore file: {e}"),
        )
    })?;

    let (address, account_kind) = match account_kind {
        AccountKind::EvmLocal => {
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

            // Write keystore file directly (it's already encrypted)
            let keys_dir = heat_core::config::HeatConfig::home_dir()?.join("keys");
            heat_core::fs::ensure_dir(&keys_dir)?;
            let dest = keys_dir.join(format!("{name}.json"));
            heat_core::fs::atomic_write_secure(&dest, content.as_bytes())?;

            (address, AccountKind::EvmLocal)
        }
        AccountKind::SolanaLocal => {
            // Solana CLI exports keypairs as JSON arrays of 64 bytes: [u8; 64]
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

            // Validate that the pubkey half matches the derived pubkey
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

            // Encrypt the 32-byte seed into Heat's keystore format
            let password =
                keystore::resolve_password(password_file, password_env)?.ok_or_else(|| {
                    HeatError::auth("no_password", "Password required for key encryption")
                        .with_hint("Use --password-file, --password-env, or set HEAT_PASSWORD")
                })?;
            keystore::save_key(name, seed, password.as_bytes(), Some(&address))?;

            (Some(address), AccountKind::SolanaLocal)
        }
    };

    let account = Account {
        name: name.to_string(),
        account_type: account_kind,
        key_name: name.to_string(),
        address,
        default_network: None,
        protocols: Default::default(),
    };
    account.save()?;

    if ctx.output.format == OutputFormat::Json {
        let info = AccountInfo::from(&account);
        ctx.output.write_data(&info, None).map_err(io_err)?;
    } else {
        ctx.output
            .diagnostic(&format!("Account '{name}' imported from {keystore_path}."));
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

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Failed to write output: {e}"))
}
