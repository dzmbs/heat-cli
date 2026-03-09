//! Solana keypair / pubkey resolution from Heat accounts.
//!
//! Heat stores keys in its V3-encrypted keystore format.  For Solana accounts
//! the stored secret is the 32-byte Ed25519 seed.  The 64-byte Solana keypair
//! (seed || public_key) is derived from that seed at load time.

use heat_core::{accounts::AccountFamily, ctx::Ctx, error::HeatError, keystore};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use zeroize::Zeroize;

/// Resolve the Solana public key for the current account context.
///
/// Does NOT require the user's password — uses the cached address stored in
/// the account metadata when available, otherwise returns an error.
pub fn resolve_pubkey(ctx: &Ctx) -> Result<Pubkey, HeatError> {
    let account = ctx.require_account()?;
    account.require_family(AccountFamily::Solana, "solana")?;

    if let Some(addr) = &account.address {
        return crate::parse::parse_pubkey(addr);
    }

    Err(HeatError::auth(
        "no_cached_address",
        "Solana account has no cached address. \
             Please re-import the account to populate it.",
    )
    .with_hint("Re-create the account with 'heat accounts create --account-type solana-local'"))
}

/// Resolve the full Solana `Keypair`, resolving the password automatically.
///
/// This is the Solana equivalent of heat-evm's `private_key_signer(ctx)`.
/// Resolves password from flags/env internally — callers do not need to handle passwords.
pub fn keypair(ctx: &Ctx) -> Result<Keypair, HeatError> {
    let account = ctx.require_account()?;
    account.require_family(AccountFamily::Solana, "solana")?;

    let password = keystore::resolve_account_password(&account)?.ok_or_else(|| {
        HeatError::auth("no_password", "Password required to decrypt Solana key")
            .with_hint("Create the account with --persist-password, use --password-file/--password-env, or set HEAT_PASSWORD")
    })?;

    let mut seed_bytes = keystore::load_key(&account.key_name, password.as_bytes())?;

    if seed_bytes.len() != 32 {
        seed_bytes.zeroize();
        return Err(HeatError::internal(
            "invalid_key_length",
            format!(
                "Expected 32-byte Ed25519 seed, got {} bytes for key '{}'",
                seed_bytes.len(),
                account.key_name
            ),
        ));
    }

    let mut seed_array = [0u8; 32];
    seed_array.copy_from_slice(&seed_bytes);
    seed_bytes.zeroize();
    let keypair = Keypair::new_from_array(seed_array);
    seed_array.zeroize();

    Ok(keypair)
}

/// Resolve the full Solana `Keypair` with an explicit password.
///
/// Use `keypair(ctx)` for the common case where password resolution
/// should be automatic. This variant exists for callers that already
/// have the password (e.g., account creation flows).
pub fn resolve_keypair(ctx: &Ctx, password: &[u8]) -> Result<Keypair, HeatError> {
    let account = ctx.require_account()?;
    account.require_family(AccountFamily::Solana, "solana")?;

    let mut seed_bytes = keystore::load_key(&account.key_name, password)?;

    if seed_bytes.len() != 32 {
        seed_bytes.zeroize();
        return Err(HeatError::internal(
            "invalid_key_length",
            format!(
                "Expected 32-byte Ed25519 seed, got {} bytes for key '{}'",
                seed_bytes.len(),
                account.key_name
            ),
        ));
    }

    let mut seed_array = [0u8; 32];
    seed_array.copy_from_slice(&seed_bytes);
    seed_bytes.zeroize();
    let keypair = Keypair::new_from_array(seed_array);
    seed_array.zeroize();

    Ok(keypair)
}
