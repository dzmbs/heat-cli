//! Minimal SPL token helpers.
//!
//! Only what Pacifica and future Solana protocol crates actually need:
//! - Associated Token Account (ATA) address derivation
//! - Token account balance query

use heat_core::error::HeatError;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;

/// Derive the Associated Token Account (ATA) address for a given owner + mint.
///
/// This is a pure, deterministic computation — no RPC call required.
/// Uses the canonical ATA program derivation rule.
pub fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(owner, mint)
}

/// Fetch the token balance (in base units) of an SPL token account.
///
/// Returns `u64` raw base units (e.g. 1_000_000 = 1 USDC with 6 decimals).
pub fn token_balance(client: &RpcClient, ata: &Pubkey) -> Result<u64, HeatError> {
    let response = client
        .get_token_account_balance(ata)
        .map_err(|e| HeatError::network("rpc_token_balance", format!("RPC error: {e}")))?;

    // `amount` is the raw u64 as a string (Solana JSON-RPC quirk).
    response
        .amount
        .parse::<u64>()
        .map_err(|e| HeatError::internal("token_balance_parse", format!("Parse error: {e}")))
}

/// Build a `spl_token::instruction::transfer` instruction.
///
/// Convenience wrapper used by protocol crates that construct SPL transfer
/// transactions without pulling in spl-token directly.
pub fn transfer_instruction(
    source_ata: &Pubkey,
    dest_ata: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Result<solana_instruction::Instruction, HeatError> {
    spl_token::instruction::transfer(
        &spl_token::id(),
        source_ata,
        dest_ata,
        authority,
        &[], // no multisig signers
        amount,
    )
    .map_err(|e| {
        HeatError::internal(
            "spl_transfer_instruction",
            format!("Failed to build SPL transfer instruction: {e}"),
        )
    })
}
