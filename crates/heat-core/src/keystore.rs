//! Ethereum V3-compatible encrypted keystore.
//! scrypt(n=8192, r=8, p=1) + AES-128-CTR.
//! Compatible with Foundry / MetaMask / geth.

use crate::config::HeatConfig;
use crate::error::HeatError;
use aes::cipher::{KeyIvInit, StreamCipher};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::path::PathBuf;
use zeroize::Zeroize;

type Aes128Ctr = ctr::Ctr128BE<aes::Aes128>;

const SCRYPT_N: u32 = 8192;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;
const SCRYPT_DKLEN: usize = 32;
const KEY_LEN: usize = 32; // 256-bit private key

// ── V3 keystore JSON structure ──

#[derive(Debug, Serialize, Deserialize)]
pub struct KeystoreFile {
    pub version: u32,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    pub crypto: CryptoSection,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CryptoSection {
    pub cipher: String,
    pub ciphertext: String,
    pub cipherparams: CipherParams,
    pub kdf: String,
    pub kdfparams: KdfParams,
    pub mac: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CipherParams {
    pub iv: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KdfParams {
    pub n: u32,
    pub r: u32,
    pub p: u32,
    pub dklen: u32,
    pub salt: String,
}

// ── Public API ──

/// Encrypt a private key and save to ~/.heat/keys/<name>.json
/// Fails if a key with this name already exists.
pub fn save_key(name: &str, private_key: &[u8], password: &[u8]) -> Result<(), HeatError> {
    if private_key.len() != KEY_LEN {
        return Err(HeatError::validation(
            "invalid_key_length",
            format!("Private key must be {KEY_LEN} bytes"),
        ));
    }

    let dir = keys_dir()?;
    crate::fs::ensure_dir(&dir)?;
    let path = dir.join(format!("{name}.json"));

    if path.exists() {
        return Err(
            HeatError::validation("key_exists", format!("Key '{name}' already exists"))
                .with_hint("Remove the existing key first, or choose a different name"),
        );
    }

    let mut keystore = encrypt(private_key, password)?;
    // Store address in keystore (lowercase hex without 0x, V3 convention)
    if let Ok(addr) = derive_evm_address(private_key) {
        keystore.address = Some(addr.strip_prefix("0x").unwrap_or(&addr).to_string());
    }
    let json = serde_json::to_string_pretty(&keystore).map_err(|e| {
        HeatError::internal(
            "keystore_serialize",
            format!("Failed to serialize keystore: {e}"),
        )
    })?;

    crate::fs::atomic_write_secure(&path, json.as_bytes())
}

/// Check whether a key with this name exists on disk.
pub fn key_exists(name: &str) -> Result<bool, HeatError> {
    Ok(keys_dir()?.join(format!("{name}.json")).exists())
}

/// Load and decrypt a private key from ~/.heat/keys/<name>.json
pub fn load_key(name: &str, password: &[u8]) -> Result<Vec<u8>, HeatError> {
    let path = keys_dir()?.join(format!("{name}.json"));
    if !path.exists() {
        return Err(
            HeatError::auth("key_not_found", format!("Key not found: {name}"))
                .with_hint("Use 'heat accounts create' to create an account with a key"),
        );
    }
    let content = std::fs::read_to_string(&path).map_err(|e| {
        HeatError::internal("key_read", format!("Failed to read keystore {name}: {e}"))
    })?;
    let keystore: KeystoreFile = serde_json::from_str(&content).map_err(|e| {
        HeatError::internal("key_parse", format!("Invalid keystore file {name}: {e}"))
    })?;
    decrypt(&keystore, password)
}

/// List all key names.
pub fn list_keys() -> Result<Vec<String>, HeatError> {
    let dir = keys_dir()?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| HeatError::internal("keys_list", format!("Failed to read keys dir: {e}")))?;
    for entry in entries {
        let entry = entry
            .map_err(|e| HeatError::internal("keys_list", format!("Failed to read entry: {e}")))?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Remove a key file.
pub fn remove_key(name: &str) -> Result<(), HeatError> {
    let path = keys_dir()?.join(format!("{name}.json"));
    if !path.exists() {
        return Err(HeatError::auth(
            "key_not_found",
            format!("Key not found: {name}"),
        ));
    }
    std::fs::remove_file(&path)
        .map_err(|e| HeatError::internal("key_remove", format!("Failed to remove key {name}: {e}")))
}

// ── Encryption / Decryption ──

fn encrypt(private_key: &[u8], password: &[u8]) -> Result<KeystoreFile, HeatError> {
    let mut salt = [0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), &mut salt);

    let mut iv = [0u8; 16];
    rand::Rng::fill(&mut rand::thread_rng(), &mut iv);

    let mut derived_key = [0u8; SCRYPT_DKLEN];
    let params = scrypt::Params::new(
        SCRYPT_N.trailing_zeros() as u8,
        SCRYPT_R,
        SCRYPT_P,
        SCRYPT_DKLEN,
    )
    .map_err(|e| HeatError::internal("scrypt_params", format!("Invalid scrypt params: {e}")))?;

    scrypt::scrypt(password, &salt, &params, &mut derived_key).map_err(|e| {
        HeatError::internal("scrypt_derive", format!("scrypt derivation failed: {e}"))
    })?;

    // AES-128-CTR: use first 16 bytes of derived key
    let enc_key = &derived_key[..16];
    let mut ciphertext = private_key.to_vec();
    let mut cipher = Aes128Ctr::new(enc_key.into(), &iv.into());
    cipher.apply_keystream(&mut ciphertext);

    // MAC: keccak256(derived_key[16..32] ++ ciphertext)
    let mut mac_input = Vec::with_capacity(16 + ciphertext.len());
    mac_input.extend_from_slice(&derived_key[16..32]);
    mac_input.extend_from_slice(&ciphertext);
    let mac = Keccak256::digest(&mac_input);

    derived_key.zeroize();

    Ok(KeystoreFile {
        version: 3,
        id: uuid::Uuid::new_v4().to_string(),
        address: None,
        crypto: CryptoSection {
            cipher: "aes-128-ctr".to_string(),
            ciphertext: hex::encode(&ciphertext),
            cipherparams: CipherParams {
                iv: hex::encode(iv),
            },
            kdf: "scrypt".to_string(),
            kdfparams: KdfParams {
                n: SCRYPT_N,
                r: SCRYPT_R,
                p: SCRYPT_P,
                dklen: SCRYPT_DKLEN as u32,
                salt: hex::encode(salt),
            },
            mac: hex::encode(mac),
        },
    })
}

fn decrypt(keystore: &KeystoreFile, password: &[u8]) -> Result<Vec<u8>, HeatError> {
    if keystore.version != 3 {
        return Err(HeatError::validation(
            "unsupported_keystore_version",
            format!("Unsupported keystore version: {}", keystore.version),
        ));
    }
    if keystore.crypto.kdf != "scrypt" {
        return Err(HeatError::validation(
            "unsupported_kdf",
            format!("Unsupported KDF: {}", keystore.crypto.kdf),
        ));
    }

    let salt = hex::decode(&keystore.crypto.kdfparams.salt)
        .map_err(|_| HeatError::internal("keystore_decode", "Invalid salt hex"))?;
    let iv = hex::decode(&keystore.crypto.cipherparams.iv)
        .map_err(|_| HeatError::internal("keystore_decode", "Invalid IV hex"))?;
    let ciphertext = hex::decode(&keystore.crypto.ciphertext)
        .map_err(|_| HeatError::internal("keystore_decode", "Invalid ciphertext hex"))?;
    let expected_mac = hex::decode(&keystore.crypto.mac)
        .map_err(|_| HeatError::internal("keystore_decode", "Invalid MAC hex"))?;

    let kp = &keystore.crypto.kdfparams;
    let mut derived_key = [0u8; SCRYPT_DKLEN];
    let params = scrypt::Params::new(kp.n.trailing_zeros() as u8, kp.r, kp.p, kp.dklen as usize)
        .map_err(|e| HeatError::internal("scrypt_params", format!("Invalid scrypt params: {e}")))?;

    scrypt::scrypt(password, &salt, &params, &mut derived_key).map_err(|e| {
        HeatError::internal("scrypt_derive", format!("scrypt derivation failed: {e}"))
    })?;

    // Verify MAC
    let mut mac_input = Vec::with_capacity(16 + ciphertext.len());
    mac_input.extend_from_slice(&derived_key[16..32]);
    mac_input.extend_from_slice(&ciphertext);
    let computed_mac = Keccak256::digest(&mac_input);

    if computed_mac.as_slice() != expected_mac.as_slice() {
        derived_key.zeroize();
        return Err(HeatError::auth(
            "wrong_password",
            "Incorrect password or corrupted keystore",
        ));
    }

    // Decrypt
    let enc_key = &derived_key[..16];
    let mut plaintext = ciphertext;
    let mut cipher = Aes128Ctr::new(enc_key.into(), iv.as_slice().into());
    cipher.apply_keystream(&mut plaintext);

    derived_key.zeroize();
    Ok(plaintext)
}

fn keys_dir() -> Result<PathBuf, HeatError> {
    Ok(HeatConfig::home_dir()?.join("keys"))
}

/// Resolve password from flags/env.
/// Precedence: --password-file > --password-env > HEAT_PASSWORD
pub fn resolve_password(
    password_file: Option<&str>,
    password_env: Option<&str>,
) -> Result<Option<String>, HeatError> {
    if let Some(path) = password_file {
        let content = std::fs::read_to_string(path).map_err(|e| {
            HeatError::auth(
                "password_file",
                format!("Failed to read password file: {e}"),
            )
        })?;
        return Ok(Some(content.trim().to_string()));
    }
    if let Some(var_name) = password_env {
        return match std::env::var(var_name) {
            Ok(val) => Ok(Some(val.trim().to_string())),
            Err(_) => Err(HeatError::auth(
                "password_env",
                format!("Environment variable {var_name} not set"),
            )),
        };
    }
    if let Ok(val) = std::env::var("HEAT_PASSWORD") {
        return Ok(Some(val.trim().to_string()));
    }
    Ok(None)
}

/// Normalize a keystore address field to 0x-prefixed lowercase.
/// Accepts with or without 0x prefix, validates 40 hex chars.
pub fn normalize_keystore_address(addr: &str) -> Result<String, HeatError> {
    let trimmed = addr.trim();
    let hex_part = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if hex_part.len() != 40 {
        return Err(HeatError::validation(
            "invalid_address",
            format!(
                "Invalid address in keystore: expected 40 hex chars, got {}",
                hex_part.len()
            ),
        ));
    }
    // Validate hex
    hex::decode(hex_part).map_err(|_| {
        HeatError::validation("invalid_address", "Address contains invalid hex characters")
    })?;
    Ok(format!("0x{}", hex_part.to_lowercase()))
}

/// Derive an EVM address from a 32-byte private key.
/// Returns the normalized lowercase hex address (0x-prefixed).
pub fn derive_evm_address(private_key: &[u8]) -> Result<String, HeatError> {
    use k256::ecdsa::SigningKey;

    let signing_key = SigningKey::from_slice(private_key)
        .map_err(|e| HeatError::validation("invalid_key", format!("Invalid private key: {e}")))?;
    let verifying_key = signing_key.verifying_key();
    // Uncompressed public key is 65 bytes (0x04 || x || y). We hash x || y (skip prefix).
    let pubkey_bytes = verifying_key.to_encoded_point(false);
    let pubkey_uncompressed = &pubkey_bytes.as_bytes()[1..]; // skip 0x04 prefix

    let hash = Keccak256::digest(pubkey_uncompressed);
    let address = &hash[12..]; // last 20 bytes
    Ok(format!("0x{}", hex::encode(address)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keystore_roundtrip() {
        let key = [0xABu8; 32];
        let password = b"test-password-123";

        let keystore = encrypt(&key, password).unwrap();
        assert_eq!(keystore.version, 3);
        assert_eq!(keystore.crypto.cipher, "aes-128-ctr");
        assert_eq!(keystore.crypto.kdf, "scrypt");

        let decrypted = decrypt(&keystore, password).unwrap();
        assert_eq!(decrypted, key);
    }

    #[test]
    fn test_wrong_password() {
        let key = [0xCDu8; 32];
        let keystore = encrypt(&key, b"correct").unwrap();
        let result = decrypt(&keystore, b"wrong");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.category, crate::error::ErrorCategory::Auth);
    }

    #[test]
    fn test_derive_evm_address() {
        // Well-known test vector: private key 1 → known address
        let mut key = [0u8; 32];
        key[31] = 1;
        let addr = derive_evm_address(&key).unwrap();
        assert_eq!(
            addr.to_lowercase(),
            "0x7e5f4552091a69125d5dfcb7b8c2659029395bdf"
        );
    }

    #[test]
    fn test_derive_evm_address_invalid_key() {
        let key = [0u8; 32]; // All zeros is invalid for secp256k1
        assert!(derive_evm_address(&key).is_err());
    }
}
