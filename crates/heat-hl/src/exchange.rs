//! Raw Hyperliquid exchange actions not yet exposed by hypersdk.
//!
//! Uses the same signing scheme as hypersdk (RMP hash → Agent EIP-712).
//! Reference: hlz/src/sdk/signing.zig, hlz/src/sdk/client.zig
//!
//! This module exists only to fill gaps in hypersdk 0.2.6.
//! When hypersdk adds native support, replace with SDK calls.

use alloy::primitives::B256;
use alloy::signers::SignerSync;
use alloy::sol;
use alloy::sol_types::{Eip712Domain, eip712_domain};
use heat_core::error::HeatError;
use hypersdk::hypercore::Chain;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

/// EIP-712 domain matching Hyperliquid's exchange.
///
/// Identical to hypersdk's CORE_MAINNET_EIP712_DOMAIN (pub(super), so we replicate).
/// Source: hypersdk/src/hypercore/types/mod.rs:139
/// Verified against: hlz/src/sdk/eip712.zig (EXCHANGE_DOMAIN)
/// Used for both mainnet and testnet — source field ("a"/"b") distinguishes them.
const DOMAIN: Eip712Domain = eip712_domain! {
    name: "Exchange",
    version: "1",
    chain_id: 1337,
    verifying_contract: alloy::primitives::Address::ZERO,
};

sol! {
    struct Agent {
        string source;
        bytes32 connectionId;
    }
}

/// UpdateLeverage action — not in hypersdk 0.2.6's Action enum.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateLeverageAction {
    #[serde(rename = "type")]
    action_type: String,
    asset: usize,
    is_cross: bool,
    leverage: u32,
}

/// Signature in Hyperliquid's JSON format.
#[derive(Serialize)]
struct SigJson {
    r: String,
    s: String,
    v: u8,
}

/// Full exchange request body.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExchangeRequest<A: Serialize> {
    action: A,
    nonce: u64,
    signature: SigJson,
    vault_address: Option<String>,
    expires_after: Option<u64>,
}

/// Exchange response.
#[derive(Deserialize)]
#[serde(tag = "status", content = "response")]
#[serde(rename_all = "camelCase")]
pub enum ExchangeResponse {
    Ok(serde_json::Value),
    Err(String),
}

/// Set leverage for an asset on Hyperliquid.
///
/// Bypasses hypersdk and constructs the signed request directly.
/// Signing scheme: RMP hash → keccak256 → Agent EIP-712 (identical to orders).
pub async fn update_leverage<S: SignerSync>(
    signer: &S,
    chain: Chain,
    base_url: &str,
    asset: usize,
    is_cross: bool,
    leverage: u32,
) -> Result<(), HeatError> {
    let action = UpdateLeverageAction {
        action_type: "updateLeverage".to_string(),
        asset,
        is_cross,
        leverage,
    };

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Step 1: RMP hash — serialize action to MessagePack, append nonce + vault flag
    let mut bytes = rmp_serde::to_vec_named(&action).map_err(|e| {
        HeatError::internal("rmp_serialize", format!("Failed to serialize action: {e}"))
    })?;
    bytes.extend(nonce.to_be_bytes());
    bytes.push(0x00); // no vault address

    let hash: [u8; 32] = Keccak256::digest(&bytes).into();
    let connection_id = B256::from(hash);

    // Step 2: Sign Agent struct via EIP-712
    // source: "a" = mainnet, "b" = testnet (hypersdk signing.rs:34, hlz eip712.zig)
    let source = if chain == Chain::Mainnet { "a" } else { "b" };
    let agent = Agent {
        source: source.to_string(),
        connectionId: connection_id,
    };

    let sig = signer
        .sign_typed_data_sync(&agent, &DOMAIN)
        .map_err(|e| HeatError::auth("sign_failed", format!("Failed to sign: {e}")))?;

    // v: alloy returns y_parity as bool (true=1, false=0), Hyperliquid expects 27/28
    let sig_json = SigJson {
        r: format!("{:#066x}", sig.r()),
        s: format!("{:#066x}", sig.s()),
        v: if sig.v() { 28 } else { 27 },
    };

    // Step 3: POST to /exchange
    let request = ExchangeRequest {
        action,
        nonce,
        signature: sig_json,
        vault_address: None,
        expires_after: None,
    };

    let url = format!("{}/exchange", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp =
        client.post(&url).json(&request).send().await.map_err(|e| {
            HeatError::network("exchange_post", format!("POST /exchange failed: {e}"))
        })?;

    let body: ExchangeResponse = resp.json().await.map_err(|e| {
        HeatError::network(
            "exchange_response",
            format!("Failed to parse exchange response: {e}"),
        )
    })?;

    match body {
        ExchangeResponse::Ok(_) => Ok(()),
        ExchangeResponse::Err(msg) => Err(HeatError::protocol(
            "update_leverage_failed",
            format!("updateLeverage rejected: {msg}"),
        )),
    }
}
