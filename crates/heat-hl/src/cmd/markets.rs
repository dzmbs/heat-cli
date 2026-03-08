use clap::Args;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use serde::Serialize;

use super::client_from_ctx;

#[derive(Args)]
pub struct PerpsArgs {
    /// Filter by DEX name (for HIP-3 perps)
    #[arg(long)]
    pub dex: Option<String>,
}

#[derive(Args)]
pub struct SpotArgs {}

#[derive(Serialize)]
struct PerpInfo {
    name: String,
    index: usize,
    sz_decimals: i64,
    max_leverage: u64,
}

#[derive(Serialize)]
struct SpotInfo {
    name: String,
    index: usize,
    base: String,
    quote: String,
}

pub async fn perps(args: PerpsArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;

    let markets = if let Some(dex_name) = &args.dex {
        let dexes = client.perp_dexs().await.map_err(net_err)?;
        let dex = dexes
            .iter()
            .find(|d| d.name().eq_ignore_ascii_case(dex_name))
            .ok_or_else(|| {
                HeatError::validation("dex_not_found", format!("DEX not found: {dex_name}"))
            })?;
        client.perps_from(dex.clone()).await.map_err(net_err)?
    } else {
        client.perps().await.map_err(net_err)?
    };

    let infos: Vec<PerpInfo> = markets
        .iter()
        .map(|m| PerpInfo {
            name: m.name.clone(),
            index: m.index,
            sz_decimals: m.sz_decimals,
            max_leverage: m.max_leverage,
        })
        .collect();

    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&infos, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            println!("{:<12} {:>6} {:>8} {:>8}", "NAME", "INDEX", "SZ_DEC", "MAX_LEV");
            for p in &infos {
                println!(
                    "{:<12} {:>6} {:>8} {:>7}x",
                    p.name, p.index, p.sz_decimals, p.max_leverage
                );
            }
        }
        OutputFormat::Quiet => {}
    }
    Ok(())
}

pub async fn spot(_args: SpotArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;
    let markets = client.spot().await.map_err(net_err)?;

    let infos: Vec<SpotInfo> = markets
        .iter()
        .map(|m| SpotInfo {
            name: m.name.clone(),
            index: m.index,
            base: m.tokens[0].name.clone(),
            quote: m.tokens[1].name.clone(),
        })
        .collect();

    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&infos, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            println!("{:<16} {:>6} {:>8} {:>8}", "NAME", "INDEX", "BASE", "QUOTE");
            for s in &infos {
                println!("{:<16} {:>6} {:>8} {:>8}", s.name, s.index, s.base, s.quote);
            }
        }
        OutputFormat::Quiet => {}
    }
    Ok(())
}

fn net_err(e: impl std::fmt::Display) -> HeatError {
    HeatError::network("hl_fetch", format!("Hyperliquid API error: {e}"))
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
