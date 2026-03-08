use clap::Args;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use serde::Serialize;

use super::client_from_ctx;
use crate::signer;

#[derive(Args)]
pub struct BalanceArgs {
    /// Address to query (defaults to account address)
    #[arg(long)]
    pub address: Option<String>,
}

#[derive(Serialize)]
struct BalanceInfo {
    coin: String,
    total: String,
    available: String,
    hold: String,
}

#[derive(Serialize)]
struct BalanceOutput {
    address: String,
    account_value: String,
    balances: Vec<BalanceInfo>,
}

pub async fn run(args: BalanceArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;

    let address = if let Some(addr) = &args.address {
        addr.parse().map_err(|_| {
            HeatError::validation("invalid_address", format!("Invalid address: {addr}"))
        })?
    } else {
        signer::resolve_address(ctx)?
    };

    let state = client
        .clearinghouse_state(address, None)
        .await
        .map_err(|e| HeatError::network("balance_fetch", format!("Failed to fetch balance: {e}")))?;

    let balances_raw = client.user_balances(address).await.map_err(|e| {
        HeatError::network("balance_fetch", format!("Failed to fetch balances: {e}"))
    })?;

    let balances: Vec<BalanceInfo> = balances_raw
        .iter()
        .filter(|b| b.total != rust_decimal::Decimal::ZERO)
        .map(|b| BalanceInfo {
            coin: b.coin.clone(),
            total: b.total.to_string(),
            available: b.available().to_string(),
            hold: b.hold.to_string(),
        })
        .collect();

    let out = BalanceOutput {
        address: format!("{address}"),
        account_value: state.margin_summary.account_value.to_string(),
        balances,
    };

    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&out, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            println!("Account: {}", out.address);
            println!("Value:   ${}", out.account_value);
            if out.balances.is_empty() {
                println!("No balances.");
            } else {
                println!();
                println!("{:<10} {:>14} {:>14} {:>14}", "COIN", "TOTAL", "AVAILABLE", "HOLD");
                for b in &out.balances {
                    println!("{:<10} {:>14} {:>14} {:>14}", b.coin, b.total, b.available, b.hold);
                }
            }
        }
        OutputFormat::Quiet => {
            ctx.output
                .write_scalar(&out.account_value)
                .map_err(io_err)?;
        }
    }
    Ok(())
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
