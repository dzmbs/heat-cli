use clap::Args;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use hypersdk::hypercore::types::Side;
use serde::Serialize;

use super::client_from_ctx;
use crate::signer;

#[derive(Args)]
pub struct OrdersArgs {
    /// Address to query (defaults to account address)
    #[arg(long)]
    pub address: Option<String>,
}

#[derive(Serialize)]
struct OrderInfo {
    oid: u64,
    coin: String,
    side: String,
    size: String,
    price: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cloid: Option<String>,
}

pub async fn run(args: OrdersArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;

    let address = if let Some(addr) = &args.address {
        addr.parse().map_err(|_| {
            HeatError::validation("invalid_address", format!("Invalid address: {addr}"))
        })?
    } else {
        signer::resolve_address(ctx)?
    };

    let open = client.open_orders(address, None).await.map_err(|e| {
        HeatError::network("orders_fetch", format!("Failed to fetch orders: {e}"))
    })?;

    let infos: Vec<OrderInfo> = open
        .iter()
        .map(|o| OrderInfo {
            oid: o.oid,
            coin: o.coin.clone(),
            side: match o.side {
                Side::Bid => "buy".to_string(),
                Side::Ask => "sell".to_string(),
            },
            size: o.sz.to_string(),
            price: o.limit_px.to_string(),
            cloid: o.cloid.map(|c| format!("{c}")),
        })
        .collect();

    match ctx.output.format {
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&infos, None).map_err(io_err)?;
        }
        OutputFormat::Pretty => {
            if infos.is_empty() {
                ctx.output.diagnostic("No open orders.");
            } else {
                println!(
                    "{:>12} {:<8} {:>6} {:>12} {:>14}",
                    "OID", "COIN", "SIDE", "SIZE", "PRICE"
                );
                for o in &infos {
                    println!(
                        "{:>12} {:<8} {:>6} {:>12} {:>14}",
                        o.oid, o.coin, o.side, o.size, o.price
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
