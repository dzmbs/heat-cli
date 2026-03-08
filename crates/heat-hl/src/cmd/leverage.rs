use clap::{Args, Subcommand};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use heat_core::safety::DryRunPreview;
use serde::Serialize;

use super::client_from_ctx;
use crate::asset;
use crate::exchange;
use crate::signer;

#[derive(Args)]
pub struct LeverageArgs {
    #[command(subcommand)]
    pub command: LeverageSubcommand,
}

#[derive(Subcommand)]
pub enum LeverageSubcommand {
    /// Show current leverage for all positions
    Show,
    /// Set leverage for an asset
    Set {
        /// Asset name (e.g., BTC, ETH)
        asset: String,
        /// Leverage multiplier (e.g., 5, 10, 20)
        value: u32,
        /// Use isolated margin instead of cross (default is cross)
        #[arg(long)]
        isolated: bool,
    },
}

#[derive(Serialize)]
struct LeverageInfo {
    asset: String,
    leverage_type: String,
    value: u32,
}

#[derive(Serialize)]
struct LeverageSetResult {
    asset: String,
    leverage: u32,
    margin_type: String,
}

pub async fn run(args: LeverageArgs, ctx: &Ctx) -> Result<(), HeatError> {
    match args.command {
        LeverageSubcommand::Show => show(ctx).await,
        LeverageSubcommand::Set {
            asset,
            isolated,
            value,
        } => set(&asset, value, !isolated, ctx).await,
    }
}

async fn show(ctx: &Ctx) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;
    let address = signer::resolve_address(ctx)?;

    let state = client
        .clearinghouse_state(address, None)
        .await
        .map_err(|e| {
            HeatError::network(
                "clearinghouse_fetch",
                format!("Failed to fetch clearinghouse state: {e}"),
            )
        })?;

    let infos: Vec<LeverageInfo> = state
        .asset_positions
        .iter()
        .filter(|ap| !ap.position.szi.is_zero())
        .map(|ap| LeverageInfo {
            asset: ap.position.coin.clone(),
            leverage_type: format!("{:?}", ap.position.leverage.leverage_type),
            value: ap.position.leverage.value,
        })
        .collect();

    if infos.is_empty() {
        ctx.output.diagnostic("No positions with leverage info.");
        return Ok(());
    }

    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&infos, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            for info in &infos {
                println!(
                    "{:<12} {}x ({})",
                    info.asset, info.value, info.leverage_type
                );
            }
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}

async fn set(asset_name: &str, leverage: u32, is_cross: bool, ctx: &Ctx) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;
    let resolved = asset::resolve(&client, asset_name).await?;
    let margin_type = if is_cross { "cross" } else { "isolated" };

    if ctx.dry_run {
        DryRunPreview::new("hl", "leverage set")
            .param("asset", &resolved.name)
            .param("leverage", &leverage.to_string())
            .param("margin", margin_type)
            .display();
        return Ok(());
    }

    ctx.confirm_dangerous(&format!(
        "set {}x {} leverage on {}",
        leverage, margin_type, resolved.name
    ))?;

    let s = signer::resolve_signer(ctx)?;
    let chain = super::chain_from_ctx(ctx)?;

    let base_url = match chain {
        hypersdk::hypercore::Chain::Mainnet => "https://api.hyperliquid.xyz",
        hypersdk::hypercore::Chain::Testnet => "https://api.hyperliquid-testnet.xyz",
    };

    exchange::update_leverage(&s, chain, base_url, resolved.index, is_cross, leverage).await?;

    let result = LeverageSetResult {
        asset: resolved.name.clone(),
        leverage,
        margin_type: margin_type.to_string(),
    };

    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&result, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            println!(
                "Leverage set to {}x ({}) for {}.",
                leverage, margin_type, resolved.name
            );
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
