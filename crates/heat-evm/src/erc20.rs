//! Minimal ERC-20 helpers using Alloy's `sol!` macro.
//!
//! This module covers read operations and one write operation (approve).
//! For protocol-level transfers, call the token contract directly from
//! the protocol crate using `wallet_provider`.

use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use alloy::sol;
use heat_core::error::HeatError;

// ERC-20 interface — minimal surface needed by Heat protocol crates.
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IERC20 {
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
        function balanceOf(address owner) external view returns (uint256);
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 amount) external returns (bool);
        function transfer(address to, uint256 amount) external returns (bool);
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
    }
}

// ── Read helpers ─────────────────────────────────────────────────────────────

/// Fetch the ERC-20 symbol string (e.g., "USDC").
pub async fn symbol(provider: impl Provider, token: Address) -> Result<String, HeatError> {
    let contract = IERC20::new(token, provider);
    contract
        .symbol()
        .call()
        .await
        .map_err(|e| HeatError::network("erc20_symbol", format!("Failed to fetch symbol: {e}")))
}

/// Fetch the ERC-20 decimals (e.g., 6 for USDC, 18 for most tokens).
pub async fn decimals(provider: impl Provider, token: Address) -> Result<u8, HeatError> {
    let contract = IERC20::new(token, provider);
    contract
        .decimals()
        .call()
        .await
        .map_err(|e| HeatError::network("erc20_decimals", format!("Failed to fetch decimals: {e}")))
}

/// Fetch the token balance of `owner`.
pub async fn balance_of(
    provider: impl Provider,
    token: Address,
    owner: Address,
) -> Result<U256, HeatError> {
    let contract = IERC20::new(token, provider);
    contract.balanceOf(owner).call().await.map_err(|e| {
        HeatError::network("erc20_balance_of", format!("Failed to fetch balance: {e}"))
    })
}

/// Fetch the allowance that `owner` has granted to `spender`.
pub async fn allowance(
    provider: impl Provider,
    token: Address,
    owner: Address,
    spender: Address,
) -> Result<U256, HeatError> {
    let contract = IERC20::new(token, provider);
    contract
        .allowance(owner, spender)
        .call()
        .await
        .map_err(|e| {
            HeatError::network("erc20_allowance", format!("Failed to fetch allowance: {e}"))
        })
}

// ── Write helper ─────────────────────────────────────────────────────────────

/// Approve `spender` to spend `amount` of `token` on behalf of the signer.
///
/// **This is a write operation.** The provider must be wallet-enabled
/// (built with `heat_evm::signer::wallet_provider`). Returns the transaction hash.
///
/// # Dry-run
/// Callers are responsible for checking `ctx.dry_run` before calling this.
pub async fn approve(
    provider: impl Provider,
    token: Address,
    spender: Address,
    amount: U256,
) -> Result<alloy::primitives::TxHash, HeatError> {
    let contract = IERC20::new(token, provider);
    let tx = contract.approve(spender, amount);
    let pending = tx.send().await.map_err(|e| {
        HeatError::network(
            "erc20_approve",
            format!("Failed to send approve transaction: {e}"),
        )
    })?;
    let receipt = pending.get_receipt().await.map_err(|e| {
        HeatError::network(
            "erc20_approve_receipt",
            format!("Failed to get approve receipt: {e}"),
        )
    })?;

    // Check receipt status — a value of false (0) means the transaction reverted.
    if !receipt.status() {
        return Err(HeatError::protocol(
            "erc20_approve_reverted",
            format!(
                "Approve transaction reverted (tx: {:#x})",
                receipt.transaction_hash
            ),
        )
        .with_hint("The token contract rejected the approval. For USDT, try approving 0 first."));
    }

    Ok(receipt.transaction_hash)
}
