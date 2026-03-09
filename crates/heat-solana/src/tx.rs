//! Transaction building and submission helpers.
//!
//! Provides thin wrappers around the Solana RPC client for the most common
//! transaction lifecycle steps that protocol crates need.

use heat_core::error::HeatError;
use solana_hash::Hash;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSimulateTransactionConfig;
use solana_signature::Signature;
use solana_transaction::Transaction;

/// Result of a transaction simulation.
#[derive(Debug)]
pub struct SimulationResult {
    /// Compute units consumed by the simulation.
    pub units_consumed: Option<u64>,
    /// Log messages produced during simulation.
    pub logs: Vec<String>,
    /// Whether the simulation succeeded (no error).
    pub success: bool,
    /// Error description if simulation failed.
    pub error: Option<String>,
}

/// Fetch the latest blockhash from the cluster.
///
/// This must be called immediately before signing a transaction, as blockhashes
/// expire after roughly 150 slots (~60–90 seconds).
pub fn fetch_blockhash(client: &RpcClient) -> Result<Hash, HeatError> {
    client
        .get_latest_blockhash()
        .map_err(|e| HeatError::network("rpc_blockhash", format!("Failed to fetch blockhash: {e}")))
}

/// Simulate a transaction and return structured results.
///
/// Useful for dry-run flows and preflight checks.  Does NOT consume lamports
/// or commit any state.
pub fn simulate_transaction(
    client: &RpcClient,
    tx: &Transaction,
) -> Result<SimulationResult, HeatError> {
    let config = RpcSimulateTransactionConfig {
        sig_verify: false,
        replace_recent_blockhash: true,
        commitment: None,
        encoding: None,
        accounts: None,
        min_context_slot: None,
        inner_instructions: false,
    };

    let response = client
        .simulate_transaction_with_config(tx, config)
        .map_err(|e| {
            HeatError::network(
                "rpc_simulate",
                format!("Transaction simulation failed: {e}"),
            )
        })?;

    let value = response.value;
    let success = value.err.is_none();
    let error = value.err.map(|e| format!("{e:?}"));
    let logs = value.logs.unwrap_or_default();
    let units_consumed = value.units_consumed;

    Ok(SimulationResult {
        units_consumed,
        logs,
        success,
        error,
    })
}

/// Send a signed transaction and wait for confirmation.
///
/// Returns the transaction `Signature` on success.
///
/// This function blocks until the transaction is confirmed or an error is
/// returned.  Protocol crates that need custom commitment levels should call
/// the RPC client directly.
pub fn send_and_confirm(client: &RpcClient, tx: &Transaction) -> Result<Signature, HeatError> {
    client
        .send_and_confirm_transaction(tx)
        .map_err(|e| HeatError::network("rpc_send", format!("Transaction failed: {e}")))
}

/// Dry-run helper: simulate a transaction and return the simulation result.
///
/// If `ctx.dry_run` is true, callers should call this instead of `send_and_confirm`.
/// Returns `Ok(SimulationResult)` on success; returns `Err` if simulation reports an error.
pub fn dry_run(client: &RpcClient, tx: &Transaction) -> Result<SimulationResult, HeatError> {
    let result = simulate_transaction(client, tx)?;
    if !result.success {
        return Err(HeatError::protocol(
            "simulation_failed",
            format!(
                "Transaction simulation failed: {}",
                result.error.as_deref().unwrap_or("unknown error")
            ),
        ));
    }
    Ok(result)
}
