use clap::{Args, Subcommand};
use futures::StreamExt;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use hypersdk::hypercore::types::{Incoming, Subscription};
use hypersdk::hypercore::ws::Event;

use super::client_from_ctx;
use crate::asset;

#[derive(Args)]
pub struct StreamArgs {
    #[command(subcommand)]
    pub command: StreamSubcommand,
}

#[derive(Subcommand)]
pub enum StreamSubcommand {
    /// Stream real-time trades for an asset
    Trades(TradesArgs),
}

#[derive(Args)]
pub struct TradesArgs {
    /// Asset name (e.g., BTC, ETH, PURR/USDC)
    pub asset: String,
}

pub async fn run(args: StreamArgs, ctx: &Ctx) -> Result<(), HeatError> {
    match args.command {
        StreamSubcommand::Trades(a) => trades(a, ctx).await,
    }
}

async fn trades(args: TradesArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let client = client_from_ctx(ctx)?;
    let resolved = asset::resolve(&client, &args.asset).await?;

    // SDK auto-reconnects with exponential backoff and re-subscribes
    let mut ws = client.websocket();
    ws.subscribe(Subscription::Trades {
        coin: resolved.name.clone(),
    });

    ctx.output
        .diagnostic(&format!("Subscribing to {} trades...", resolved.name));

    while let Some(event) = ws.next().await {
        match event {
            Event::Connected => {
                ctx.output.diagnostic("Connected");
            }
            Event::Disconnected => {
                // SDK handles reconnection internally with exponential backoff
                ctx.output
                    .diagnostic("Disconnected (SDK auto-reconnecting)");
            }
            Event::Message(msg) => match msg {
                Incoming::Trades(trades) => {
                    for trade in &trades {
                        match ctx.output.format {
                            OutputFormat::Pretty => {
                                println!(
                                    "{} {} {} @ {}",
                                    trade.coin, trade.side, trade.sz, trade.px,
                                );
                            }
                            OutputFormat::Json | OutputFormat::Ndjson => {
                                let _ = ctx.output.write_ndjson(trade);
                            }
                            OutputFormat::Quiet => {}
                        }
                    }
                }
                Incoming::SubscriptionResponse(_) => {
                    ctx.output.diagnostic("Subscription confirmed");
                }
                _ => {}
            },
        }
    }

    Ok(())
}
