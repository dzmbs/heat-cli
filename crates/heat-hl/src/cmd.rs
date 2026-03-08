mod balance;
mod cancel;
mod leverage;
mod markets;
mod order;
mod orders;
mod positions;
mod price;
mod send;
mod stream;

use clap::{Args, Subcommand};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use hypersdk::hypercore::{self, Chain, HttpClient};

/// Chain selection for Hyperliquid.
/// Rejects unknown network values to prevent silent mainnet fallback.
fn chain_from_ctx(ctx: &Ctx) -> Result<Chain, HeatError> {
    match ctx.network.as_deref() {
        None | Some("mainnet") => Ok(Chain::Mainnet),
        Some("testnet") => Ok(Chain::Testnet),
        Some(other) => Err(HeatError::validation(
            "invalid_network",
            format!("Unknown network: {other}"),
        )
        .with_hint("Valid networks for Hyperliquid: mainnet, testnet")),
    }
}

fn client_from_ctx(ctx: &Ctx) -> Result<HttpClient, HeatError> {
    Ok(match chain_from_ctx(ctx)? {
        Chain::Mainnet => hypercore::mainnet(),
        Chain::Testnet => hypercore::testnet(),
    })
}

#[derive(Args)]
pub struct HlCmd {
    #[command(subcommand)]
    pub command: HlSubcommand,
}

#[derive(Subcommand)]
pub enum HlSubcommand {
    /// Show price for an asset
    Price(price::PriceArgs),
    /// List perpetual markets
    Perps(markets::PerpsArgs),
    /// List spot markets
    Spot(markets::SpotArgs),
    /// Show account balances
    Balance(balance::BalanceArgs),
    /// Show open positions
    Positions(positions::PositionsArgs),
    /// Show open orders
    Orders(orders::OrdersArgs),
    /// Place a buy order
    Buy(order::BuyArgs),
    /// Place a sell order
    Sell(order::SellArgs),
    /// Cancel orders
    Cancel(cancel::CancelArgs),
    /// View or set leverage
    Leverage(leverage::LeverageArgs),
    /// Send USDC or spot tokens to another address
    Send(send::SendArgs),
    /// Stream real-time market data
    Stream(stream::StreamArgs),
}

pub async fn run(cmd: HlCmd, ctx: &Ctx) -> Result<(), HeatError> {
    match cmd.command {
        HlSubcommand::Price(args) => price::run(args, ctx).await,
        HlSubcommand::Perps(args) => markets::perps(args, ctx).await,
        HlSubcommand::Spot(args) => markets::spot(args, ctx).await,
        HlSubcommand::Balance(args) => balance::run(args, ctx).await,
        HlSubcommand::Positions(args) => positions::run(args, ctx).await,
        HlSubcommand::Orders(args) => orders::run(args, ctx).await,
        HlSubcommand::Buy(args) => order::buy(args, ctx).await,
        HlSubcommand::Sell(args) => order::sell(args, ctx).await,
        HlSubcommand::Cancel(args) => cancel::run(args, ctx).await,
        HlSubcommand::Leverage(args) => leverage::run(args, ctx).await,
        HlSubcommand::Send(args) => send::run(args, ctx).await,
        HlSubcommand::Stream(args) => stream::run(args, ctx).await,
    }
}
