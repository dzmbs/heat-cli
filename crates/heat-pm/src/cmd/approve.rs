//! Approve commands — ERC20/ERC1155 token approval management.

use alloy::primitives::{Address, U256};
use alloy::sol;
use clap::Subcommand;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use heat_core::safety::DryRunPreview;
use polymarket_client_sdk::POLYGON;
use polymarket_client_sdk::contract_config;
use serde::Serialize;

use crate::auth;

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}

// --- ABI interfaces ---

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 amount) external returns (bool);
    }
}

sol! {
    #[sol(rpc)]
    interface IERC1155 {
        function isApprovedForAll(address account, address operator) external view returns (bool);
        function setApprovalForAll(address operator, bool approved) external;
    }
}

// --- Output DTOs ---

#[derive(Serialize)]
struct ApprovalTarget {
    name: String,
    address: String,
    erc20_allowance: String,
    erc20_approved: bool,
    erc1155_approved: bool,
}

#[derive(Serialize)]
struct ApprovalStatus {
    owner: String,
    targets: Vec<ApprovalTarget>,
}

#[derive(Serialize)]
struct ApprovalSetTarget {
    name: String,
    erc20_tx: String,
    erc1155_tx: String,
}

#[derive(Serialize)]
struct ApprovalSetResult {
    targets: Vec<ApprovalSetTarget>,
}

// --- Subcommand ---

#[derive(Subcommand)]
pub enum ApproveSubcommand {
    /// Check current approval status for all Polymarket exchange contracts
    Check,
    /// Set max approval for all Polymarket exchange contracts
    Set,
}

// --- Helpers ---

fn rpc_err(op: &str, e: impl std::fmt::Display) -> HeatError {
    HeatError::network("rpc_call", format!("{op} failed: {e}"))
}

fn tx_err(op: &str, e: impl std::fmt::Display) -> HeatError {
    HeatError::network("tx_send", format!("{op} transaction failed: {e}"))
}

/// Returns the three approval targets: standard CTF exchange, neg-risk exchange,
/// and neg-risk adapter (if present in the config).
fn approval_targets() -> Result<Vec<(&'static str, Address)>, HeatError> {
    let config = contract_config(POLYGON, false).ok_or_else(|| {
        HeatError::protocol("no_contract_config", "No contract config found for Polygon")
    })?;
    let neg_risk_config = contract_config(POLYGON, true).ok_or_else(|| {
        HeatError::protocol(
            "no_neg_risk_config",
            "No neg-risk contract config found for Polygon",
        )
    })?;

    let mut targets: Vec<(&'static str, Address)> = vec![
        ("CTF Exchange", config.exchange),
        ("Neg Risk Exchange", neg_risk_config.exchange),
    ];

    if let Some(adapter) = neg_risk_config.neg_risk_adapter {
        targets.push(("Neg Risk Adapter", adapter));
    }

    Ok(targets)
}

// --- Command handler ---

pub async fn run(sub: ApproveSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    match sub {
        ApproveSubcommand::Check => check(ctx).await,
        ApproveSubcommand::Set => set(ctx).await,
    }
}

async fn check(ctx: &Ctx) -> Result<(), HeatError> {
    let owner: Address = auth::resolve_eoa_address(ctx)?;

    let config = contract_config(POLYGON, false).ok_or_else(|| {
        HeatError::protocol("no_contract_config", "No contract config found for Polygon")
    })?;
    let collateral = config.collateral;
    let conditional_tokens = config.conditional_tokens;

    let targets = approval_targets()?;
    let provider = auth::readonly_provider().await?;

    let erc20 = IERC20::new(collateral, &provider);
    let erc1155 = IERC1155::new(conditional_tokens, &provider);

    // Threshold: consider ERC20 approved if allowance >= 2^128
    let threshold = U256::from(1u64) << 128;

    let mut target_statuses: Vec<ApprovalTarget> = Vec::with_capacity(targets.len());

    for (name, target_addr) in &targets {
        let allowance: U256 = erc20
            .allowance(owner, *target_addr)
            .call()
            .await
            .map_err(|e| rpc_err(&format!("ERC20.allowance({name})"), e))?;

        let erc20_approved = allowance >= threshold;

        let erc1155_approved: bool = erc1155
            .isApprovedForAll(owner, *target_addr)
            .call()
            .await
            .map_err(|e| rpc_err(&format!("ERC1155.isApprovedForAll({name})"), e))?;

        target_statuses.push(ApprovalTarget {
            name: name.to_string(),
            address: format!("{target_addr}"),
            erc20_allowance: allowance.to_string(),
            erc20_approved,
            erc1155_approved,
        });
    }

    let status = ApprovalStatus {
        owner: format!("{owner}"),
        targets: target_statuses,
    };

    match ctx.output.format {
        OutputFormat::Pretty => {
            println!("Owner: {}", status.owner);
            println!();
            for t in &status.targets {
                println!("  {} ({})", t.name, t.address);
                println!(
                    "    USDC allowance:  {} ({})",
                    t.erc20_allowance,
                    if t.erc20_approved {
                        "approved"
                    } else {
                        "NOT approved"
                    }
                );
                println!(
                    "    CTF approved:    {}",
                    if t.erc1155_approved {
                        "approved"
                    } else {
                        "NOT approved"
                    }
                );
            }

            let any_missing = status
                .targets
                .iter()
                .any(|t| !t.erc20_approved || !t.erc1155_approved);
            if any_missing {
                ctx.output
                    .diagnostic("Run 'heat pm approve set' to grant missing approvals");
            }
        }
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&status, None).map_err(io_err)?;
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}

async fn set(ctx: &Ctx) -> Result<(), HeatError> {
    if ctx.dry_run {
        DryRunPreview::new("pm", "approve set").display();
        return Ok(());
    }

    ctx.confirm_dangerous("set max approval for all Polymarket exchange contracts")?;

    let targets = approval_targets()?;

    let config = contract_config(POLYGON, false).ok_or_else(|| {
        HeatError::protocol("no_contract_config", "No contract config found for Polygon")
    })?;
    let collateral = config.collateral;
    let conditional_tokens = config.conditional_tokens;

    let provider = auth::wallet_provider(ctx).await?;

    let erc20 = IERC20::new(collateral, &provider);
    let erc1155 = IERC1155::new(conditional_tokens, &provider);

    let mut set_targets: Vec<ApprovalSetTarget> = Vec::with_capacity(targets.len());

    for (name, target_addr) in &targets {
        let erc20_receipt = erc20
            .approve(*target_addr, U256::MAX)
            .send()
            .await
            .map_err(|e| tx_err(&format!("ERC20.approve({name})"), e))?
            .get_receipt()
            .await
            .map_err(|e| tx_err(&format!("ERC20.approve({name}) receipt"), e))?;

        let erc1155_receipt = erc1155
            .setApprovalForAll(*target_addr, true)
            .send()
            .await
            .map_err(|e| tx_err(&format!("ERC1155.setApprovalForAll({name})"), e))?
            .get_receipt()
            .await
            .map_err(|e| tx_err(&format!("ERC1155.setApprovalForAll({name}) receipt"), e))?;

        set_targets.push(ApprovalSetTarget {
            name: name.to_string(),
            erc20_tx: format!("{}", erc20_receipt.transaction_hash),
            erc1155_tx: format!("{}", erc1155_receipt.transaction_hash),
        });
    }

    let result = ApprovalSetResult {
        targets: set_targets,
    };

    match ctx.output.format {
        OutputFormat::Pretty => {
            for t in &result.targets {
                println!("{}:", t.name);
                println!("  ERC20 approve tx:          {}", t.erc20_tx);
                println!("  ERC1155 setApprovalForAll: {}", t.erc1155_tx);
            }
        }
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&result, None).map_err(io_err)?;
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}
