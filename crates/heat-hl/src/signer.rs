//! Bridge heat accounts to hypersdk signers.

use alloy::signers::local::PrivateKeySigner;
use heat_core::accounts::Account;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::keystore;
use hypersdk::Address;

/// Resolve a signer from the heat account system.
/// Requires password to decrypt the key — use only for write commands.
pub fn resolve_signer(ctx: &Ctx) -> Result<PrivateKeySigner, HeatError> {
    let account = ctx.require_account()?;

    let password = keystore::resolve_password(None, None)?.ok_or_else(|| {
        HeatError::auth("no_password", "Password required to decrypt key")
            .with_hint("Set HEAT_PASSWORD env var")
    })?;

    let key_bytes = keystore::load_key(&account.key_name, password.as_bytes())?;

    let signer = PrivateKeySigner::from_slice(&key_bytes).map_err(|e| {
        HeatError::auth(
            "invalid_key",
            format!("Failed to create signer from key: {e}"),
        )
    })?;

    Ok(signer)
}

/// Derive the address from a signer.
pub fn signer_address(signer: &PrivateKeySigner) -> Address {
    alloy::signers::Signer::address(signer)
}

/// Resolve account address without decrypting the key (for read-only commands).
/// Falls back to signer derivation if address not cached in account metadata,
/// and backfills the cache for future calls.
pub fn resolve_address(ctx: &Ctx) -> Result<Address, HeatError> {
    let account = ctx.require_account()?;

    // Use cached address if available
    if let Some(addr_str) = &account.address {
        return addr_str.parse().map_err(|_| {
            HeatError::internal(
                "invalid_cached_address",
                format!("Cached address is invalid: {addr_str}"),
            )
        });
    }

    // Fallback: decrypt key and derive (for imported accounts without cached address)
    let signer = resolve_signer(ctx)?;
    let address = signer_address(&signer);

    // Backfill: save address to account metadata so future reads don't need password
    backfill_address(&account.name, &format!("{address}"));

    Ok(address)
}

/// Best-effort backfill of address into account metadata.
/// Silently ignores errors — this is an optimization, not critical.
fn backfill_address(account_name: &str, address: &str) {
    if let Ok(mut account) = Account::load(account_name) {
        if account.address.is_none() {
            account.address = Some(address.to_string());
            // Account::save() fails if file exists, so we need to overwrite
            let _ = account.save_update();
        }
    }
}
