//! Bridge heat accounts to Polymarket SDK signers.
//!
//! Heat reuses the same EVM keystore for Polymarket. The signature_type
//! (proxy/eoa/gnosis-safe) is stored in account.protocols.polymarket.

use std::str::FromStr;

use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::keystore;
use polymarket_client_sdk::auth::Normal;
use polymarket_client_sdk::auth::Signer as _;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::clob::types::SignatureType;
use polymarket_client_sdk::{POLYGON, clob, derive_proxy_wallet, derive_safe_wallet};

pub const RPC_URL: &str = "https://polygon.drpc.org";

fn parse_signature_type(s: &str) -> Result<SignatureType, HeatError> {
    match s {
        "proxy" => Ok(SignatureType::Proxy),
        "eoa" => Ok(SignatureType::Eoa),
        "gnosis-safe" => Ok(SignatureType::GnosisSafe),
        _ => Err(HeatError::validation(
            "invalid_signature_type",
            format!("Unknown Polymarket signature type: {s}"),
        )
        .with_hint("Valid types: proxy, eoa, gnosis-safe")),
    }
}

/// Resolve signature type from account config or flag.
/// Precedence: flag > account.protocols.polymarket.signature_type > "proxy"
pub fn resolve_signature_type(ctx: &Ctx, flag: Option<&str>) -> Result<SignatureType, HeatError> {
    let raw = if let Some(st) = flag {
        st.to_string()
    } else if let Some(name) = &ctx.account_name {
        if let Ok(account) = heat_core::accounts::Account::load(name) {
            if let Some(pm_config) = account.protocols.get("polymarket") {
                if let Some(st) = pm_config.get("signature_type").and_then(|v| v.as_str()) {
                    st.to_string()
                } else {
                    "proxy".to_string()
                }
            } else {
                "proxy".to_string()
            }
        } else {
            "proxy".to_string()
        }
    } else {
        "proxy".to_string()
    };
    parse_signature_type(&raw)
}

/// Resolve a PrivateKeySigner from the heat account system.
pub fn resolve_signer(ctx: &Ctx) -> Result<PrivateKeySigner, HeatError> {
    let account = ctx.require_account()?;

    let password = keystore::resolve_password(None, None)?.ok_or_else(|| {
        HeatError::auth("no_password", "Password required to decrypt key")
            .with_hint("Set HEAT_PASSWORD env var")
    })?;

    let key_bytes = keystore::load_key(&account.key_name, password.as_bytes())?;
    let hex_key = format!("0x{}", hex::encode(&key_bytes));

    let signer = PrivateKeySigner::from_str(&hex_key).map_err(|e| {
        HeatError::auth("invalid_key", format!("Failed to create signer: {e}"))
    })?;

    Ok(signer.with_chain_id(Some(POLYGON)))
}

/// Resolve the EOA address without decrypting key (for read-only commands).
/// This is the signer/EOA address, NOT the effective Polymarket trading wallet.
pub fn resolve_eoa_address(ctx: &Ctx) -> Result<Address, HeatError> {
    let account = ctx.require_account()?;

    if let Some(addr) = &account.address {
        return addr.parse().map_err(|_| {
            HeatError::validation("invalid_address", format!("Invalid account address: {addr}"))
        });
    }

    // Fallback: decrypt and derive
    let signer = resolve_signer(ctx)?;
    Ok(signer.address())
}

/// Resolve the effective Polymarket trading wallet address.
///
/// For proxy mode: derives the proxy wallet from EOA using CREATE2
/// For gnosis-safe mode: derives the safe wallet from EOA
/// For EOA mode: returns the signer address directly
///
/// This is the address that holds positions, balances, and trades on Polymarket.
pub fn resolve_pm_address(ctx: &Ctx, sig_type_flag: Option<&str>) -> Result<String, HeatError> {
    let eoa = resolve_eoa_address(ctx)?;
    let sig_type = resolve_signature_type(ctx, sig_type_flag)?;

    match sig_type {
        SignatureType::Proxy => {
            let proxy = derive_proxy_wallet(eoa, POLYGON).ok_or_else(|| {
                HeatError::protocol(
                    "proxy_derivation_failed",
                    "Could not derive proxy wallet for Polygon",
                )
            })?;
            Ok(format!("{proxy}"))
        }
        SignatureType::GnosisSafe => {
            let safe = derive_safe_wallet(eoa, POLYGON).ok_or_else(|| {
                HeatError::protocol(
                    "safe_derivation_failed",
                    "Could not derive Safe wallet for Polygon",
                )
            })?;
            Ok(format!("{safe}"))
        }
        SignatureType::Eoa => Ok(format!("{eoa}")),
        _ => Err(HeatError::validation(
            "unsupported_signature_type",
            "Unsupported Polymarket signature type",
        )),
    }
}

/// Create an authenticated CLOB client.
pub async fn authenticated_clob_client(
    ctx: &Ctx,
    sig_type_flag: Option<&str>,
) -> Result<clob::Client<Authenticated<Normal>>, HeatError> {
    let signer = resolve_signer(ctx)?;
    let sig_type = resolve_signature_type(ctx, sig_type_flag)?;

    clob::Client::default()
        .authentication_builder(&signer)
        .signature_type(sig_type)
        .authenticate()
        .await
        .map_err(|e| {
            HeatError::auth(
                "clob_auth_failed",
                format!("Failed to authenticate with Polymarket CLOB: {e}"),
            )
        })
}

/// Create an alloy provider for on-chain calls (read-only).
pub async fn readonly_provider(
) -> Result<impl alloy::providers::Provider + Clone, HeatError> {
    ProviderBuilder::new()
        .connect(RPC_URL)
        .await
        .map_err(|e| HeatError::network("rpc_connect", format!("Failed to connect to Polygon RPC: {e}")))
}

/// Create an alloy provider with wallet for on-chain transactions.
pub async fn wallet_provider(
    ctx: &Ctx,
) -> Result<impl alloy::providers::Provider + Clone, HeatError> {
    let signer = resolve_signer(ctx)?;
    ProviderBuilder::new()
        .wallet(signer)
        .connect(RPC_URL)
        .await
        .map_err(|e| {
            HeatError::network(
                "rpc_connect",
                format!("Failed to connect to Polygon RPC with wallet: {e}"),
            )
        })
}

#[cfg(test)]
mod tests {
    use alloy::primitives::address;
    use polymarket_client_sdk::{POLYGON, derive_proxy_wallet, derive_safe_wallet};

    use super::parse_signature_type;

    // ── parse_signature_type ─────────────────────────────────────────────

    #[test]
    fn parse_signature_type_valid_proxy() {
        let result = parse_signature_type("proxy");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_signature_type_valid_eoa() {
        let result = parse_signature_type("eoa");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_signature_type_valid_gnosis_safe() {
        let result = parse_signature_type("gnosis-safe");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_signature_type_rejects_wrong_case() {
        assert!(parse_signature_type("Proxy").is_err());
        assert!(parse_signature_type("EOA").is_err());
        assert!(parse_signature_type("GnosisSafe").is_err());
        assert!(parse_signature_type("GNOSIS-SAFE").is_err());
    }

    #[test]
    fn parse_signature_type_rejects_unknown() {
        assert!(parse_signature_type("foo").is_err());
        assert!(parse_signature_type("").is_err());
        assert!(parse_signature_type("metamask").is_err());
    }

    #[test]
    fn parse_signature_type_error_has_hint() {
        let err = parse_signature_type("bad").unwrap_err();
        let debug_str = format!("{err:?}");
        // The error carries the hint "Valid types: proxy, eoa, gnosis-safe"
        assert!(debug_str.contains("proxy") || format!("{err}").contains("proxy"),
            "error should mention valid types in its hint");
    }

    // ── wallet derivation ────────────────────────────────────────────────

    // EOA: 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 (Foundry/Anvil test key #0)
    // These expected addresses are cross-checked against the SDK's own test suite.

    #[test]
    fn proxy_wallet_differs_from_eoa() {
        let eoa = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let proxy = derive_proxy_wallet(eoa, POLYGON).expect("derivation should succeed on Polygon");
        assert_ne!(eoa, proxy, "proxy wallet must differ from the signing EOA");
    }

    #[test]
    fn safe_wallet_differs_from_eoa() {
        let eoa = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let safe = derive_safe_wallet(eoa, POLYGON).expect("derivation should succeed on Polygon");
        assert_ne!(eoa, safe, "safe wallet must differ from the signing EOA");
    }

    #[test]
    fn safe_wallet_differs_from_proxy_wallet() {
        let eoa = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let proxy = derive_proxy_wallet(eoa, POLYGON).unwrap();
        let safe = derive_safe_wallet(eoa, POLYGON).unwrap();
        assert_ne!(proxy, safe, "proxy and safe wallet addresses must be distinct");
    }

    #[test]
    fn proxy_wallet_polygon_known_address() {
        let eoa = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let proxy = derive_proxy_wallet(eoa, POLYGON).unwrap();
        assert_eq!(
            proxy,
            address!("0x365f0cA36ae1F641E02Fe3b7743673DA42A13a70"),
            "proxy wallet address must be deterministic"
        );
    }

    #[test]
    fn safe_wallet_polygon_known_address() {
        let eoa = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let safe = derive_safe_wallet(eoa, POLYGON).unwrap();
        assert_eq!(
            safe,
            address!("0xd93b25Cb943D14d0d34FBAf01fc93a0F8b5f6e47"),
            "safe wallet address must be deterministic"
        );
    }

    #[test]
    fn proxy_wallet_unsupported_chain_returns_none() {
        // Chain ID 1 (Ethereum mainnet) has no Polymarket proxy factory config.
        let eoa = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        assert!(
            derive_proxy_wallet(eoa, 1).is_none(),
            "unsupported chain should return None"
        );
    }
}
