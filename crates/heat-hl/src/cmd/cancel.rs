use clap::Args;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use heat_core::safety::DryRunPreview;
use hypersdk::hypercore::types::{BatchCancel, Cancel};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use super::client_from_ctx;
use crate::asset;
use crate::signer;

#[derive(Args)]
pub struct CancelArgs {
    /// Asset name (required unless --all)
    pub asset: Option<String>,
    /// Cancel by order ID
    #[arg(long)]
    pub oid: Option<u64>,
    /// Cancel all open orders for asset (or all assets if no asset given)
    #[arg(long)]
    pub all: bool,
}

#[derive(Serialize)]
struct CancelResult {
    cancelled: usize,
    status: String,
}

pub async fn run(args: CancelArgs, ctx: &Ctx) -> Result<(), HeatError> {
    if !args.all && args.oid.is_none() {
        return Err(HeatError::validation(
            "cancel_target",
            "Must specify --oid or --all",
        )
        .with_hint("heat hl cancel BTC --oid 12345  or  heat hl cancel --all"));
    }

    let client = client_from_ctx(ctx)?;
    let s = signer::resolve_signer(ctx)?;
    let address = signer::signer_address(&s);

    if let Some(oid) = args.oid {
        // Cancel single order by OID
        let asset_name = args.asset.as_deref().ok_or_else(|| {
            HeatError::validation("cancel_asset", "Asset required when cancelling by --oid")
        })?;
        let resolved = asset::resolve(&client, asset_name).await?;

        if ctx.dry_run {
            DryRunPreview::new("hl", "cancel")
                .param("asset", &resolved.name)
                .param("oid", &oid.to_string())
                .display();
            return Ok(());
        }
        ctx.confirm_dangerous(&format!("cancel order {oid} on {}", resolved.name))?;

        let nonce = nonce_now();
        let cancel = BatchCancel {
            cancels: vec![Cancel {
                asset: resolved.index,
                oid,
            }],
        };
        let responses = client
            .cancel(&s, cancel, nonce, None, None)
            .await
            .map_err(|e| HeatError::protocol("cancel_failed", format!("Cancel failed: {e}")))?;

        emit_result(ctx, 1, &format!("{responses:?}"))
    } else {
        // Cancel all
        let open = client.open_orders(address, None).await.map_err(|e| {
            HeatError::network("orders_fetch", format!("Failed to fetch orders: {e}"))
        })?;

        let to_cancel: Vec<_> = if let Some(asset_name) = &args.asset {
            let upper = asset_name.to_uppercase();
            open.iter()
                .filter(|o| o.coin.to_uppercase() == upper)
                .collect()
        } else {
            open.iter().collect()
        };

        if to_cancel.is_empty() {
            ctx.output.diagnostic("No open orders to cancel.");
            return Ok(());
        }

        if ctx.dry_run {
            let mut preview = DryRunPreview::new("hl", "cancel --all");
            preview = preview.param("orders", &to_cancel.len().to_string());
            if let Some(a) = &args.asset {
                preview = preview.param("asset", a);
            }
            preview.display();
            return Ok(());
        }
        ctx.confirm_dangerous(&format!("cancel {} open orders", to_cancel.len()))?;

        // Resolve coin names to asset indices
        // BasicOrder has coin (String) but Cancel needs asset (usize)
        // Build a coin→index map from perps + spot + dex perps
        let perps = client.perps().await.map_err(|e| {
            HeatError::network("perps_fetch", format!("Failed to fetch perps: {e}"))
        })?;
        let spots = client.spot().await.map_err(|e| {
            HeatError::network("spot_fetch", format!("Failed to fetch spots: {e}"))
        })?;
        // Also fetch HIP-3 DEX perps (best-effort, warn on failure)
        let dexes = match client.perp_dexs().await {
            Ok(d) => d,
            Err(e) => {
                ctx.output.diagnostic(&format!("Warning: failed to fetch DEX list: {e}"));
                vec![]
            }
        };
        let mut dex_perps = Vec::new();
        for dex in &dexes {
            match client.perps_from(dex.clone()).await {
                Ok(dp) => dex_perps.extend(dp),
                Err(e) => {
                    ctx.output.diagnostic(&format!(
                        "Warning: failed to fetch perps for DEX '{}': {e}",
                        dex.name()
                    ));
                }
            }
        }

        let coin_to_index = |coin: &str| -> Option<usize> {
            for p in &perps {
                if p.name.eq_ignore_ascii_case(coin) {
                    return Some(p.index);
                }
            }
            for s in &spots {
                if s.name.eq_ignore_ascii_case(coin) {
                    return Some(s.index);
                }
            }
            for p in &dex_perps {
                if p.name.eq_ignore_ascii_case(coin) {
                    return Some(p.index);
                }
            }
            None
        };

        // Check for unresolvable orders upfront
        let mut skipped: Vec<String> = Vec::new();
        let mut resolved_cancels: Vec<Cancel> = Vec::new();
        for o in &to_cancel {
            if let Some(idx) = coin_to_index(&o.coin) {
                resolved_cancels.push(Cancel { asset: idx, oid: o.oid });
            } else {
                skipped.push(format!("{} (oid {})", o.coin, o.oid));
            }
        }

        if !skipped.is_empty() {
            ctx.output.diagnostic(&format!(
                "Warning: {} order(s) could not be resolved: {}",
                skipped.len(),
                skipped.join(", ")
            ));
        }

        if resolved_cancels.is_empty() {
            ctx.output.diagnostic("No orders could be resolved for cancellation.");
            return Ok(());
        }

        // Batch in groups of 64 (Hyperliquid limit)
        let mut total_cancelled = 0;
        for chunk in resolved_cancels.chunks(64) {
            let nonce = nonce_now();
            let batch = BatchCancel { cancels: chunk.to_vec() };
            client
                .cancel(&s, batch, nonce, None, None)
                .await
                .map_err(|e| {
                    HeatError::protocol("cancel_failed", format!("Cancel batch failed: {e}"))
                })?;
            total_cancelled += chunk.len();
        }

        emit_result(ctx, total_cancelled, "ok")
    }
}

fn emit_result(ctx: &Ctx, count: usize, status: &str) -> Result<(), HeatError> {
    let result = CancelResult {
        cancelled: count,
        status: status.to_string(),
    };
    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&result, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            println!("Cancelled {} order(s).", count);
        }
        OutputFormat::Quiet => {}
    }
    Ok(())
}

fn nonce_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
