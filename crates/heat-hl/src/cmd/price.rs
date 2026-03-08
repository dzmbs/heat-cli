use clap::Args;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use serde::Serialize;

use super::client_from_ctx;

#[derive(Args)]
pub struct PriceArgs {
    /// Asset name (e.g., BTC, ETH, PURR/USDC)
    pub asset: String,
}

#[derive(Serialize)]
struct PriceOutput {
    asset: String,
    mid: String,
}

pub async fn run(args: PriceArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;
    let mids = client.all_mids(None).await.map_err(|e| {
        HeatError::network("mids_fetch", format!("Failed to fetch prices: {e}"))
    })?;

    let upper = args.asset.to_uppercase();
    let mid = mids
        .iter()
        .find(|(k, _)| k.to_uppercase() == upper)
        .map(|(_, v)| v);

    let Some(price) = mid else {
        return Err(HeatError::validation(
            "asset_not_found",
            format!("No price found for: {}", args.asset),
        )
        .with_hint("Use 'heat hl perps' or 'heat hl spot' to list available markets"));
    };

    let out = PriceOutput {
        asset: args.asset.clone(),
        mid: price.to_string(),
    };

    match ctx.output.format {
        OutputFormat::Quiet => ctx.output.write_scalar(&price.to_string()).map_err(io_err)?,
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&out, None).map_err(io_err)?
        }
        OutputFormat::Pretty => {
            println!("{}: {}", out.asset, out.mid);
        }
    }
    Ok(())
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
