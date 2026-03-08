//! Bridge commands — deposit to Polymarket, check status.

use clap::Subcommand;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use polymarket_client_sdk::bridge;
use polymarket_client_sdk::bridge::types::{
    DepositRequest, DepositTransactionStatus, StatusRequest,
};
use serde::Serialize;

use crate::auth;

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}

fn bridge_err(e: impl std::fmt::Display) -> HeatError {
    HeatError::network("bridge_request", format!("Bridge API error: {e}"))
}

fn deposit_status_label(s: &DepositTransactionStatus) -> &'static str {
    match s {
        DepositTransactionStatus::DepositDetected => "deposit_detected",
        DepositTransactionStatus::Processing => "processing",
        DepositTransactionStatus::OriginTxConfirmed => "origin_tx_confirmed",
        DepositTransactionStatus::Submitted => "submitted",
        DepositTransactionStatus::Completed => "completed",
        DepositTransactionStatus::Failed => "failed",
        _ => "unknown",
    }
}

// ── Heat-owned output DTOs ───────────────────────────────────────────────

#[derive(Serialize)]
struct DepositInfo {
    evm: String,
    svm: String,
    btc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Serialize)]
struct DepositStatusInfo {
    transactions: Vec<DepositTxInfo>,
}

#[derive(Serialize)]
struct DepositTxInfo {
    from_chain_id: String,
    from_token_address: String,
    from_amount: String,
    to_chain_id: String,
    to_token_address: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tx_hash: Option<String>,
}

#[derive(Serialize)]
struct SupportedAssetInfo {
    chain_id: String,
    chain_name: String,
    token_name: String,
    token_symbol: String,
    min_usd: String,
}

// ── Commands ─────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum BridgeSubcommand {
    /// Deposit to Polymarket
    Deposit {
        /// Override deposit address (defaults to current account)
        #[arg(long)]
        address: Option<String>,
    },
    /// Check deposit status
    Status {
        /// Address to check
        #[arg(long)]
        address: Option<String>,
    },
    /// List supported bridge assets
    SupportedAssets,
}

pub async fn run(sub: BridgeSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    let client = bridge::Client::default();

    match sub {
        BridgeSubcommand::Deposit { address } => {
            let addr_str = match address {
                Some(a) => a,
                None => auth::resolve_pm_address(ctx, None)?,
            };
            let addr = addr_str.parse().map_err(|_| {
                HeatError::validation("invalid_address", format!("Invalid address: {addr_str}"))
            })?;

            // Deposit is a read-only command (fetches deposit addresses), no confirmation needed.
            let req = DepositRequest::builder().address(addr).build();
            let resp = client.deposit(&req).await.map_err(bridge_err)?;

            let info = DepositInfo {
                evm: format!("{}", resp.address.evm),
                svm: resp.address.svm.clone(),
                btc: resp.address.btc.clone(),
                note: resp.note.clone(),
            };

            match ctx.output.format {
                OutputFormat::Pretty => {
                    println!("EVM: {}", info.evm);
                    println!("SVM: {}", info.svm);
                    println!("BTC: {}", info.btc);
                    if let Some(note) = &info.note {
                        println!("Note: {note}");
                    }
                }
                OutputFormat::Json | OutputFormat::Ndjson => {
                    ctx.output.write_data(&info, None).map_err(io_err)?;
                }
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        BridgeSubcommand::Status { address } => {
            let addr_str = match address {
                Some(a) => a,
                None => auth::resolve_pm_address(ctx, None)?,
            };
            let req = StatusRequest::builder().address(&addr_str).build();
            let resp = client.status(&req).await.map_err(bridge_err)?;

            let info = DepositStatusInfo {
                transactions: resp
                    .transactions
                    .iter()
                    .map(|tx| DepositTxInfo {
                        from_chain_id: tx.from_chain_id.to_string(),
                        from_token_address: tx.from_token_address.clone(),
                        from_amount: tx.from_amount_base_unit.to_string(),
                        to_chain_id: tx.to_chain_id.to_string(),
                        to_token_address: format!("{}", tx.to_token_address),
                        status: deposit_status_label(&tx.status).to_string(),
                        tx_hash: tx.tx_hash.clone(),
                    })
                    .collect(),
            };

            match ctx.output.format {
                OutputFormat::Pretty => {
                    if info.transactions.is_empty() {
                        println!("No deposit transactions found.");
                    } else {
                        for tx in &info.transactions {
                            println!(
                                "{} chain:{} → chain:{} {}",
                                tx.status, tx.from_chain_id, tx.to_chain_id, tx.from_amount
                            );
                            if let Some(hash) = &tx.tx_hash {
                                println!("  tx: {hash}");
                            }
                        }
                    }
                }
                OutputFormat::Json | OutputFormat::Ndjson => {
                    ctx.output.write_data(&info, None).map_err(io_err)?;
                }
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        BridgeSubcommand::SupportedAssets => {
            let resp = client.supported_assets().await.map_err(bridge_err)?;

            let assets: Vec<SupportedAssetInfo> = resp
                .supported_assets
                .iter()
                .map(|a| SupportedAssetInfo {
                    chain_id: a.chain_id.to_string(),
                    chain_name: a.chain_name.clone(),
                    token_name: a.token.name.clone(),
                    token_symbol: a.token.symbol.clone(),
                    min_usd: a.min_checkout_usd.to_string(),
                })
                .collect();

            match ctx.output.format {
                OutputFormat::Pretty => {
                    for a in &assets {
                        println!(
                            "{:<10} {:<8} {} (min: ${} USD)",
                            a.chain_name, a.token_symbol, a.token_name, a.min_usd
                        );
                    }
                }
                OutputFormat::Json | OutputFormat::Ndjson => {
                    ctx.output.write_data(&assets, None).map_err(io_err)?;
                }
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use polymarket_client_sdk::bridge::types::DepositTransactionStatus;

    use super::deposit_status_label;

    // ── deposit_status_label ─────────────────────────────────────────────

    /// Verify that status labels are stable snake_case strings, not Debug output.
    /// If a variant produced "DepositDetected" (Debug) instead of "deposit_detected",
    /// downstream consumers relying on the label would break silently.
    #[test]
    fn deposit_status_labels_are_stable_snake_case() {
        assert_eq!(
            deposit_status_label(&DepositTransactionStatus::DepositDetected),
            "deposit_detected"
        );
        assert_eq!(
            deposit_status_label(&DepositTransactionStatus::Processing),
            "processing"
        );
        assert_eq!(
            deposit_status_label(&DepositTransactionStatus::OriginTxConfirmed),
            "origin_tx_confirmed"
        );
        assert_eq!(
            deposit_status_label(&DepositTransactionStatus::Submitted),
            "submitted"
        );
        assert_eq!(
            deposit_status_label(&DepositTransactionStatus::Completed),
            "completed"
        );
        assert_eq!(
            deposit_status_label(&DepositTransactionStatus::Failed),
            "failed"
        );
    }

    #[test]
    fn deposit_status_labels_are_all_lowercase() {
        let statuses = [
            DepositTransactionStatus::DepositDetected,
            DepositTransactionStatus::Processing,
            DepositTransactionStatus::OriginTxConfirmed,
            DepositTransactionStatus::Submitted,
            DepositTransactionStatus::Completed,
            DepositTransactionStatus::Failed,
        ];
        for status in &statuses {
            let label = deposit_status_label(status);
            assert_eq!(
                label,
                label.to_lowercase(),
                "status label '{label}' must be all lowercase"
            );
        }
    }

    #[test]
    fn deposit_status_labels_contain_no_debug_formatting() {
        // Debug output of enum variants would be PascalCase like "DepositDetected".
        // Ensure none of the stable labels contain uppercase letters.
        let statuses = [
            DepositTransactionStatus::DepositDetected,
            DepositTransactionStatus::Processing,
            DepositTransactionStatus::OriginTxConfirmed,
            DepositTransactionStatus::Submitted,
            DepositTransactionStatus::Completed,
            DepositTransactionStatus::Failed,
        ];
        for status in &statuses {
            let label = deposit_status_label(status);
            let debug_repr = format!("{status:?}");
            assert_ne!(
                label, debug_repr,
                "label '{label}' must not be the Debug representation '{debug_repr}'"
            );
        }
    }

    /// Verify the Deposit command does NOT trigger a dangerous confirmation.
    ///
    /// Bridge deposit is a read-only command: it fetches bridge deposit addresses
    /// via a GET request. No funds move. No on-chain transaction is submitted.
    /// Therefore no safety confirmation is needed or expected.
    ///
    /// This test documents that invariant by checking the production code path
    /// (lines 96–131 of bridge.rs) for any call to the danger-confirmation API.
    /// If you are extending bridge with a real on-chain flow, update this test
    /// and add the appropriate confirmation.
    #[test]
    fn deposit_command_is_not_marked_dangerous() {
        // Extract only the production code — everything before the #[cfg(test)] block —
        // to avoid false positives from assertion strings in the test module itself.
        let full_source = include_str!("bridge.rs");
        let prod_code = full_source
            .split("#[cfg(test)]")
            .next()
            .unwrap_or(full_source);

        assert!(
            !prod_code.contains("confirm_dangerous"),
            "production bridge code must not call confirm_dangerous: deposit is read-only"
        );
    }
}
