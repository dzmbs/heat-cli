use clap::{Args, Subcommand};
use heat_core::accounts::{self, Account, AccountType};
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
    /// Create a new EVM account from a private key
    Create {
        /// Account name
        name: String,
        /// Hex-encoded private key (0x-prefixed)
        #[arg(long)]
        key: String,
        /// Password file for key encryption
        #[arg(long)]
        password_file: Option<String>,
        /// Environment variable containing password
        #[arg(long)]
        password_env: Option<String>,
    },
    /// Import an existing V3 keystore file
    Import {
        /// Account name
        name: String,
        /// Path to V3 keystore JSON file
        #[arg(long)]
        keystore: String,
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
            account_type: match a.account_type {
                AccountType::EvmLocal => "evm-local".to_string(),
            },
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
            key,
            password_file,
            password_env,
        } => create(&name, &key, password_file.as_deref(), password_env.as_deref(), ctx),
        AccountsSubcommand::Import { name, keystore } => import(&name, &keystore, ctx),
        AccountsSubcommand::Use { name } => use_account(&name, ctx),
        AccountsSubcommand::Remove { name } => remove(&name, ctx),
    }
}

fn list(ctx: &Ctx) -> Result<(), HeatError> {
    let names = Account::list()?;
    if names.is_empty() {
        if ctx.output.format == OutputFormat::Json {
            ctx.output.write_data(&Vec::<AccountInfo>::new(), None).map_err(io_err)?;
        } else {
            ctx.output.diagnostic("No accounts. Use 'heat accounts create' to create one.");
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
    key_hex: &str,
    password_file: Option<&str>,
    password_env: Option<&str>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    // Preflight: check both before writing either
    preflight_name(name)?;

    let hex_str = key_hex.strip_prefix("0x").unwrap_or(key_hex);
    let key_bytes = hex::decode(hex_str).map_err(|_| {
        HeatError::validation("invalid_key", "Private key must be valid hex")
    })?;
    if key_bytes.len() != 32 {
        return Err(HeatError::validation(
            "invalid_key_length",
            "Private key must be 32 bytes (64 hex chars)",
        ));
    }

    let password = keystore::resolve_password(password_file, password_env)?
        .ok_or_else(|| {
            HeatError::auth("no_password", "Password required for key encryption")
                .with_hint("Use --password-file, --password-env, or set HEAT_PASSWORD")
        })?;

    // Derive address from private key before encrypting
    let address = keystore::derive_evm_address(&key_bytes)?;

    // Write key first, then account metadata
    keystore::save_key(name, &key_bytes, password.as_bytes())?;

    let account = Account {
        name: name.to_string(),
        account_type: AccountType::EvmLocal,
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

fn import(name: &str, keystore_path: &str, ctx: &Ctx) -> Result<(), HeatError> {
    // Preflight: check both before writing either
    preflight_name(name)?;

    let content = std::fs::read_to_string(keystore_path).map_err(|e| {
        HeatError::validation(
            "keystore_read",
            format!("Failed to read keystore file: {e}"),
        )
    })?;
    let parsed: heat_core::keystore::KeystoreFile = serde_json::from_str(&content).map_err(|e| {
        HeatError::validation(
            "keystore_parse",
            format!("Invalid V3 keystore file: {e}"),
        )
    })?;

    // Extract address from keystore if present
    let address = parsed.address.as_deref()
        .map(keystore::normalize_keystore_address)
        .transpose()?;

    // Write key file, then account metadata
    let keys_dir = heat_core::config::HeatConfig::home_dir()?.join("keys");
    heat_core::fs::ensure_dir(&keys_dir)?;
    let dest = keys_dir.join(format!("{name}.json"));
    heat_core::fs::atomic_write_secure(&dest, content.as_bytes())?;

    let account = Account {
        name: name.to_string(),
        account_type: AccountType::EvmLocal,
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
        ctx.output.diagnostic(&format!("Account '{name}' imported from {keystore_path}."));
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
        ctx.output.diagnostic(&format!("Default account set to '{name}'."));
    }
    Ok(())
}

fn remove(name: &str, ctx: &Ctx) -> Result<(), HeatError> {
    Account::remove(name)?;
    if ctx.output.format == OutputFormat::Json {
        ctx.output
            .write_data(&serde_json::json!({"removed": name}), None)
            .map_err(io_err)?;
    } else {
        ctx.output.diagnostic(&format!("Account '{name}' removed. Key file preserved in ~/.heat/keys/."));
    }
    Ok(())
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
        return Err(HeatError::validation(
            "key_exists",
            format!("Key '{name}' already exists"),
        )
        .with_hint("Remove the existing key first, or choose a different name"));
    }
    Ok(())
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Failed to write output: {e}"))
}
