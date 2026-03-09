//! EVM signer and provider resolution for Heat accounts.
//!
//! Centralizes the pattern from heat-hl/src/signer.rs so all EVM protocol
//! crates share the same key-decryption and provider-wiring logic.

use crate::chains::EvmChain;
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use heat_core::accounts::{Account, AccountFamily};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::keystore;

// ── Address resolution ──────────────────────────────────────────────────────

/// Resolve the EOA address for the current account without decrypting the key
/// when possible (reads cached address from account metadata).
///
/// Falls back to key decryption + derivation for accounts that have no cached
/// address, and backfills the cache as a side effect.
pub fn resolve_eoa_address(ctx: &Ctx) -> Result<Address, HeatError> {
    let account = ctx.require_account()?;
    account.require_family(AccountFamily::Evm, "evm")?;

    if let Some(addr_str) = &account.address {
        return addr_str.parse::<Address>().map_err(|_| {
            HeatError::internal(
                "invalid_cached_address",
                format!("Cached address is not a valid EVM address: '{addr_str}'"),
            )
        });
    }

    // No cached address — decrypt key and derive
    let signer = signer_from_account(&account)?;
    let address = alloy::signers::Signer::address(&signer);

    backfill_address(&account.name, &format!("{address:#x}"));

    Ok(address)
}

// ── Signer ──────────────────────────────────────────────────────────────────

/// Build a `PrivateKeySigner` from the current account's encrypted key.
/// This requires the password — use only for write (signing) operations.
pub fn private_key_signer(ctx: &Ctx) -> Result<PrivateKeySigner, HeatError> {
    let account = ctx.require_account()?;
    account.require_family(AccountFamily::Evm, "evm")?;
    signer_from_account(&account)
}

// ── Providers ───────────────────────────────────────────────────────────────

/// Build a read-only HTTP provider for the given chain/RPC URL.
///
/// Verifies that the RPC endpoint's chain ID matches `chain` so callers
/// cannot accidentally use a provider pointed at the wrong network.
pub async fn read_provider(
    chain: EvmChain,
    rpc_url: &str,
) -> Result<impl Provider + Clone, HeatError> {
    let provider = ProviderBuilder::new().connect(rpc_url).await.map_err(|e| {
        HeatError::network(
            "rpc_connect",
            format!("Failed to connect to RPC at '{rpc_url}': {e}"),
        )
    })?;

    let chain_id = provider.get_chain_id().await.map_err(|e| {
        HeatError::network(
            "rpc_chain_id",
            format!("Failed to fetch chain ID from RPC: {e}"),
        )
    })?;

    if chain_id != chain.chain_id() {
        return Err(HeatError::validation(
            "chain_id_mismatch",
            format!(
                "RPC returned chain ID {chain_id}, expected {} for {}",
                chain.chain_id(),
                chain.canonical_name()
            ),
        )
        .with_hint("Check your RPC URL — it may point to the wrong network"));
    }

    Ok(provider)
}

/// Build a wallet-enabled provider for the current account on the given chain.
///
/// Returns `impl Provider` — callers can use it directly with any Alloy API
/// that accepts a `Provider`.
pub async fn wallet_provider(
    ctx: &Ctx,
    chain: EvmChain,
    rpc_url: &str,
) -> Result<impl Provider, HeatError> {
    let signer = private_key_signer(ctx)?;

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect(rpc_url)
        .await
        .map_err(|e| {
            HeatError::network(
                "rpc_connect",
                format!("Failed to connect to RPC at '{rpc_url}': {e}"),
            )
        })?;

    // Verify chain ID matches expectation — catches misconfigured RPC endpoints.
    let chain_id = provider.get_chain_id().await.map_err(|e| {
        HeatError::network(
            "rpc_chain_id",
            format!("Failed to fetch chain ID from RPC: {e}"),
        )
    })?;

    if chain_id != chain.chain_id() {
        return Err(HeatError::validation(
            "chain_id_mismatch",
            format!(
                "RPC returned chain ID {chain_id}, expected {} for {}",
                chain.chain_id(),
                chain.canonical_name()
            ),
        )
        .with_hint("Check your RPC URL — it may point to the wrong network"));
    }

    Ok(provider)
}

// ── Internal helpers ────────────────────────────────────────────────────────

/// Decrypt and build a signer from an account's stored key.
fn signer_from_account(account: &Account) -> Result<PrivateKeySigner, HeatError> {
    let password = keystore::resolve_password(None, None)?.ok_or_else(|| {
        HeatError::auth("no_password", "Password required to decrypt key")
            .with_hint("Set the HEAT_PASSWORD environment variable")
    })?;

    let key_bytes = keystore::load_key(&account.key_name, password.as_bytes())?;

    PrivateKeySigner::from_slice(&key_bytes).map_err(|e| {
        HeatError::auth(
            "invalid_key",
            format!(
                "Failed to create signer from key '{}': {e}",
                account.key_name
            ),
        )
    })
}

/// Best-effort backfill of the derived address into account metadata.
/// Emits a warning to stderr on save failure — this is a caching optimization, not critical path.
fn backfill_address(account_name: &str, address: &str) {
    if let Ok(mut account) = Account::load(account_name)
        && account.address.is_none()
    {
        account.address = Some(address.to_string());
        if let Err(e) = account.save_update() {
            eprintln!("heat: warning: failed to cache address for account '{account_name}': {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heat_core::accounts::AccountKind;

    #[test]
    fn require_family_accepts_evm_local() {
        let account = Account {
            name: "test".to_string(),
            account_type: AccountKind::EvmLocal,
            key_name: "test".to_string(),
            address: None,
            default_network: None,
            protocols: Default::default(),
        };
        assert!(account.require_family(AccountFamily::Evm, "evm").is_ok());
    }

    #[test]
    fn signer_from_account_requires_password() {
        // SAFETY: test-only env mutation
        unsafe { std::env::remove_var("HEAT_PASSWORD") };
        let account = Account {
            name: "test".to_string(),
            account_type: AccountKind::EvmLocal,
            key_name: "nonexistent_key_heat_evm_test".to_string(),
            address: None,
            default_network: None,
            protocols: Default::default(),
        };
        let err = signer_from_account(&account).unwrap_err();
        assert_eq!(err.reason, "no_password");
    }
}
