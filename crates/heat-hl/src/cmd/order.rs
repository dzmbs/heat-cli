use clap::Args;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use heat_core::safety::DryRunPreview;
use hypersdk::hypercore::types::{
    BatchOrder, OrderGrouping, OrderRequest, OrderTypePlacement, TimeInForce,
};
use hypersdk::hypercore::{Cloid, HttpClient, PerpMarket};
use rust_decimal::Decimal;
use serde::Serialize;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use super::client_from_ctx;
use crate::asset::{self, truncate_size};
use crate::signer;

#[derive(Args)]
pub struct BuyArgs {
    /// Asset name (e.g., BTC, ETH)
    pub asset: String,
    /// Order size
    pub size: String,
    /// Limit price (omit for market order)
    #[arg(long)]
    pub price: Option<String>,
    /// Time in force: gtc, ioc, alo
    #[arg(long, default_value = "gtc")]
    pub tif: String,
    /// Reduce-only order
    #[arg(long)]
    pub reduce_only: bool,
}

#[derive(Args)]
pub struct SellArgs {
    /// Asset name (e.g., BTC, ETH)
    pub asset: String,
    /// Order size
    pub size: String,
    /// Limit price (omit for market order)
    #[arg(long)]
    pub price: Option<String>,
    /// Time in force: gtc, ioc, alo
    #[arg(long, default_value = "gtc")]
    pub tif: String,
    /// Reduce-only order
    #[arg(long)]
    pub reduce_only: bool,
}

pub async fn buy(args: BuyArgs, ctx: &Ctx) -> Result<(), HeatError> {
    place_order(ctx, &args.asset, &args.size, args.price.as_deref(), &args.tif, args.reduce_only, true).await
}

pub async fn sell(args: SellArgs, ctx: &Ctx) -> Result<(), HeatError> {
    place_order(ctx, &args.asset, &args.size, args.price.as_deref(), &args.tif, args.reduce_only, false).await
}

#[derive(Serialize)]
struct OrderResult {
    asset: String,
    side: String,
    size: String,
    price: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    oid: Option<u64>,
    status: String,
}

async fn place_order(
    ctx: &Ctx,
    asset_name: &str,
    size_str: &str,
    price_str: Option<&str>,
    tif_str: &str,
    reduce_only: bool,
    is_buy: bool,
) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;
    let resolved = asset::resolve(&client, asset_name).await?;

    let raw_size = Decimal::from_str(size_str).map_err(|_| {
        HeatError::validation("invalid_size", format!("Invalid size: {size_str}"))
    })?;
    if raw_size <= Decimal::ZERO {
        return Err(HeatError::validation("non_positive_size", "Size must be positive"));
    }
    let size = truncate_size(raw_size, resolved.sz_decimals);

    let (limit_px, order_type) = if let Some(px_str) = price_str {
        let px = Decimal::from_str(px_str).map_err(|_| {
            HeatError::validation("invalid_price", format!("Invalid price: {px_str}"))
        })?;
        let tif = parse_tif(tif_str)?;
        let rounded = round_price(&client, &resolved, px).await?;
        (rounded, OrderTypePlacement::Limit { tif })
    } else {
        // Market order — use FrontendMarket with 0.5% slippage from mid
        let mids = client.all_mids(None).await.map_err(|e| {
            HeatError::network("mids_fetch", format!("Failed to fetch prices: {e}"))
        })?;
        let mid = mids
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&resolved.name))
            .map(|(_, v)| *v)
            .ok_or_else(|| {
                HeatError::validation("no_mid", format!("No mid price for {}", resolved.name))
            })?;

        let slippage = Decimal::from_str("0.005").unwrap();
        let slippage_px = if is_buy {
            mid * (Decimal::ONE + slippage)
        } else {
            mid * (Decimal::ONE - slippage)
        };
        let rounded = round_price(&client, &resolved, slippage_px).await?;
        (rounded, OrderTypePlacement::Limit { tif: TimeInForce::FrontendMarket })
    };

    // Min notional preflight ($10)
    let notional = size * limit_px;
    let min_notional = Decimal::from(10);
    if notional < min_notional {
        let min_size = (min_notional / limit_px).ceil();
        return Err(HeatError::validation(
            "below_min_notional",
            format!("Order value ${notional} below $10 minimum"),
        )
        .with_hint(format!("Need at least {min_size} {}", resolved.name)));
    }

    let side_str = if is_buy { "buy" } else { "sell" };

    if ctx.dry_run {
        DryRunPreview::new("hl", side_str)
            .param("asset", &resolved.name)
            .param("size", &size.to_string())
            .param("price", &limit_px.to_string())
            .param("notional", &format!("${notional}"))
            .param("reduce_only", &reduce_only.to_string())
            .display();
        return Ok(());
    }

    ctx.confirm_dangerous(&format!("{side_str} {} {} @ {}", size, resolved.name, limit_px))?;

    let s = signer::resolve_signer(ctx)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let order = OrderRequest {
        asset: resolved.index,
        is_buy,
        limit_px,
        sz: size,
        reduce_only,
        order_type,
        cloid: Cloid::ZERO,
    };
    let batch = BatchOrder {
        orders: vec![order],
        grouping: OrderGrouping::Na,
    };

    let responses = client
        .place(&s, batch, nonce, None, None)
        .await
        .map_err(|e| HeatError::protocol("order_failed", format!("Order placement failed: {e}")))?;

    let resp = responses.first().ok_or_else(|| {
        HeatError::protocol("order_failed", "No response from exchange")
    })?;

    // Fail on rejected orders
    if let Some(err_msg) = resp.error() {
        return Err(HeatError::protocol(
            "order_rejected",
            format!("Order rejected: {err_msg}"),
        ));
    }

    let result = OrderResult {
        asset: resolved.name.clone(),
        side: side_str.to_string(),
        size: size.to_string(),
        price: limit_px.to_string(),
        oid: resp.oid(),
        status: format!("{resp:?}"),
    };

    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&result, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            if let Some(oid) = result.oid {
                println!("{} {} {} @ {} — oid: {oid}", side_str, result.size, result.asset, result.price);
            } else {
                println!("{} {} {} @ {} — {}", side_str, result.size, result.asset, result.price, result.status);
            }
        }
        OutputFormat::Quiet => {
            if let Some(oid) = result.oid {
                ctx.output.write_scalar(&oid.to_string()).map_err(io_err)?;
            }
        }
    }
    Ok(())
}

fn parse_tif(s: &str) -> Result<TimeInForce, HeatError> {
    match s.to_lowercase().as_str() {
        "gtc" => Ok(TimeInForce::Gtc),
        "ioc" => Ok(TimeInForce::Ioc),
        "alo" => Ok(TimeInForce::Alo),
        other => Err(HeatError::validation(
            "invalid_tif",
            format!("Invalid time-in-force: {other}"),
        )
        .with_hint("Valid values: gtc, ioc, alo")),
    }
}

async fn round_price(
    client: &HttpClient,
    resolved: &asset::ResolvedAsset,
    price: Decimal,
) -> Result<Decimal, HeatError> {
    let perps: Vec<PerpMarket> = client.perps().await.map_err(|e| {
        HeatError::network("perps_fetch", format!("Failed to fetch perps: {e}"))
    })?;
    for p in &perps {
        if p.index == resolved.index {
            return p.round_price(price).ok_or_else(|| {
                HeatError::validation("tick_round", format!("Cannot round price {price} to valid tick"))
            });
        }
    }
    // Fallback: return as-is (spot or HIP-3 where tick rounding may differ)
    Ok(price)
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
