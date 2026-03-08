use clap::Args;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use serde::Serialize;

use super::client_from_ctx;
use crate::signer;

#[derive(Args)]
pub struct PositionsArgs {
    /// Address to query (defaults to account address)
    #[arg(long)]
    pub address: Option<String>,
}

#[derive(Serialize)]
struct PositionInfo {
    coin: String,
    size: String,
    entry_px: String,
    position_value: String,
    unrealized_pnl: String,
    leverage: String,
    liquidation_px: String,
}

pub async fn run(args: PositionsArgs, ctx: &Ctx) -> Result<(), HeatError> {
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
        .map_err(|e| {
            HeatError::network("positions_fetch", format!("Failed to fetch positions: {e}"))
        })?;

    let positions: Vec<PositionInfo> = state
        .asset_positions
        .iter()
        .filter(|ap| ap.position.szi != rust_decimal::Decimal::ZERO)
        .map(|ap| {
            let p = &ap.position;
            PositionInfo {
                coin: p.coin.clone(),
                size: p.szi.to_string(),
                entry_px: p.entry_px.map(|d| d.to_string()).unwrap_or_default(),
                position_value: p.position_value.to_string(),
                unrealized_pnl: p.unrealized_pnl.to_string(),
                leverage: format!("{}x", p.leverage.value),
                liquidation_px: p.liquidation_px.map(|d| d.to_string()).unwrap_or_default(),
            }
        })
        .collect();

    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&positions, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            if positions.is_empty() {
                ctx.output.diagnostic("No open positions.");
            } else {
                println!(
                    "{:<8} {:>10} {:>12} {:>14} {:>12} {:>6} {:>12}",
                    "COIN", "SIZE", "ENTRY", "VALUE", "PNL", "LEV", "LIQ"
                );
                for p in &positions {
                    println!(
                        "{:<8} {:>10} {:>12} {:>14} {:>12} {:>6} {:>12}",
                        p.coin, p.size, p.entry_px, p.position_value, p.unrealized_pnl, p.leverage, p.liquidation_px
                    );
                }
            }
        }
        OutputFormat::Quiet => {}
    }
    Ok(())
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
