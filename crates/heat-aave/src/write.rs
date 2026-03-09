/// On-chain write operations for Aave V3.
///
/// Both `supply` and `withdraw` follow the same safety pattern:
/// 1. Resolve signer and provider via `heat-evm`
/// 2. Resolve asset (symbol → address)
/// 3. Parse exact amount
/// 4. Check/set ERC-20 approval (supply only)
/// 5. Dry-run check
/// 6. TTY confirmation
/// 7. Execute
use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use heat_core::error::HeatError;

use crate::contracts::IPool;

// ---------------------------------------------------------------------------
// Supply
// ---------------------------------------------------------------------------

/// Execute an Aave V3 supply transaction.
///
/// The caller is responsible for:
/// - resolving the provider (wallet-enabled)
/// - resolving the asset address, symbol, decimals
/// - parsing the exact amount
/// - handling ERC-20 approval beforehand
/// - dry-run / confirmation checks
///
/// Returns the transaction hash.
pub async fn supply(
    provider: impl Provider,
    pool_address: Address,
    asset: Address,
    amount: U256,
    on_behalf_of: Address,
) -> Result<alloy::primitives::TxHash, HeatError> {
    let pool = IPool::new(pool_address, &provider);

    let tx = pool.supply(asset, amount, on_behalf_of, 0u16);
    let pending = tx.send().await.map_err(|e| {
        HeatError::network(
            "aave_supply_send",
            format!("Failed to send supply transaction: {e}"),
        )
    })?;

    let receipt = pending.get_receipt().await.map_err(|e| {
        HeatError::network(
            "aave_supply_receipt",
            format!("Failed to get supply receipt: {e}"),
        )
    })?;

    if !receipt.status() {
        return Err(HeatError::protocol(
            "aave_supply_reverted",
            format!(
                "Supply transaction reverted (tx: {:#x})",
                receipt.transaction_hash
            ),
        ));
    }

    Ok(receipt.transaction_hash)
}

// ---------------------------------------------------------------------------
// Withdraw
// ---------------------------------------------------------------------------

/// Execute an Aave V3 withdraw transaction.
///
/// Same caller responsibilities as `supply`.
/// Returns the transaction hash.
pub async fn withdraw(
    provider: impl Provider,
    pool_address: Address,
    asset: Address,
    amount: U256,
    to: Address,
) -> Result<alloy::primitives::TxHash, HeatError> {
    let pool = IPool::new(pool_address, &provider);

    let tx = pool.withdraw(asset, amount, to);
    let pending = tx.send().await.map_err(|e| {
        HeatError::network(
            "aave_withdraw_send",
            format!("Failed to send withdraw transaction: {e}"),
        )
    })?;

    let receipt = pending.get_receipt().await.map_err(|e| {
        HeatError::network(
            "aave_withdraw_receipt",
            format!("Failed to get withdraw receipt: {e}"),
        )
    })?;

    if !receipt.status() {
        return Err(HeatError::protocol(
            "aave_withdraw_reverted",
            format!(
                "Withdraw transaction reverted (tx: {:#x})",
                receipt.transaction_hash
            ),
        ));
    }

    Ok(receipt.transaction_hash)
}
