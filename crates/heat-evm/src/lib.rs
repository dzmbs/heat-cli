//! `heat-evm` — shared EVM substrate crate for Heat protocol crates.
//!
//! Provides thin, composable helpers for EVM chain support:
//! - Chain metadata and parsing (`chains`)
//! - RPC URL resolution with Heat precedence rules (`rpc`)
//! - Signer and provider construction from Heat accounts (`signer`)
//! - ERC-20 read/write helpers (`erc20`)
//! - CLI argument parsing helpers (`parse`)
//! - Exact decimal ↔ base-unit conversions (`amount`)
//!
//! # Dependency direction
//! ```text
//! heat-lifi, heat-aave, ...
//!       └──▶ heat-evm
//!                 └──▶ heat-core
//! ```
//! `heat-core` MUST NOT depend on `heat-evm`.

pub mod amount;
pub mod chains;
pub mod erc20;
pub mod parse;
pub mod rpc;
pub mod signer;

// Re-export the most commonly used types so callers can write
// `heat_evm::EvmChain` instead of `heat_evm::chains::EvmChain`.
pub use chains::EvmChain;
pub use signer::{private_key_signer, read_provider, resolve_eoa_address, wallet_provider};
