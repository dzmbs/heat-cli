//! `heat-solana` — shared Solana substrate helpers for Heat CLI protocol crates.
//!
//! This crate provides thin, reusable Solana primitives:
//! - Cluster enumeration and RPC URL resolution
//! - Keypair / pubkey resolution from Heat accounts
//! - Pubkey parsing helpers
//! - SPL token utilities (ATA derivation, token balance)
//! - Transaction building / submission helpers
//! - Exact lamport ↔ decimal amount conversions
//!
//! Architecture constraints:
//! - Depends on `heat-core` for `HeatError`, `Ctx`, etc.
//! - `heat-core` must NEVER depend on this crate.
//! - `heat-evm` and `heat-solana` must NOT depend on each other.

pub mod amount;
pub mod cluster;
pub mod parse;
pub mod rpc;
pub mod signer;
pub mod spl;
pub mod tx;

// Re-export the most commonly used items so protocol crates can `use heat_solana::*`
// without needing to know the internal module layout.
pub use amount::{format_units, parse_units};
pub use cluster::SolanaCluster;
pub use parse::parse_pubkey;
pub use rpc::resolve_rpc_url;
pub use signer::{keypair, resolve_keypair, resolve_pubkey};
