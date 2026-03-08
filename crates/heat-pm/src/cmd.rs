mod approve;
mod bridge;
mod clob;
mod ctf;
mod data;
mod gamma;

use ::clap::{Args, Subcommand};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use rust_decimal::Decimal;

#[derive(Args)]
pub struct PmCmd {
    #[command(subcommand)]
    pub command: PmSubcommand,
}

#[derive(Subcommand)]
pub enum PmSubcommand {
    // ── Helper commands (primary user-facing path) ────────────────────
    /// Get price for a token
    Price {
        token_id: String,
        #[arg(long)]
        side: String,
    },

    /// Place a limit buy order
    Buy {
        token_id: String,
        #[arg(long)]
        price: Decimal,
        #[arg(long)]
        size: Decimal,
        #[arg(long)]
        sig_type: Option<String>,
    },

    /// Place a limit sell order
    Sell {
        token_id: String,
        #[arg(long)]
        price: Decimal,
        #[arg(long)]
        size: Decimal,
        #[arg(long)]
        sig_type: Option<String>,
    },

    /// Cancel an order
    Cancel {
        order_id: String,
        #[arg(long)]
        sig_type: Option<String>,
    },

    /// List open orders
    Orders {
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        sig_type: Option<String>,
    },

    /// List recent trades
    Trades {
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        sig_type: Option<String>,
    },

    /// List open positions
    Positions {
        #[arg(long)]
        user: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: i32,
    },

    /// Check balance and allowance
    Balance {
        #[arg(long)]
        sig_type: Option<String>,
    },

    // ── Protocol-native command trees (escape hatches) ────────────────
    /// Market & event data (Gamma API)
    #[command(subcommand)]
    Markets(gamma::MarketsSubcommand),

    /// Event data
    #[command(subcommand)]
    Events(gamma::EventsSubcommand),

    /// Tag browsing
    #[command(subcommand)]
    Tags(gamma::TagsSubcommand),

    /// Series data
    #[command(subcommand)]
    Series(gamma::SeriesSubcommand),

    /// Comment browsing
    #[command(subcommand)]
    Comments(gamma::CommentsSubcommand),

    /// User profiles
    #[command(subcommand)]
    Profiles(gamma::ProfilesSubcommand),

    /// Sports data
    #[command(subcommand)]
    Sports(gamma::SportsSubcommand),

    /// CLOB trading & pricing
    #[command(subcommand)]
    Clob(clob::ClobSubcommand),

    /// Conditional token operations
    #[command(subcommand)]
    Ctf(ctf::CtfSubcommand),

    /// On-chain analytics & data
    #[command(subcommand)]
    Data(data::DataSubcommand),

    /// Cross-chain bridge
    #[command(subcommand)]
    Bridge(bridge::BridgeSubcommand),

    /// Token approval management
    #[command(subcommand)]
    Approve(approve::ApproveSubcommand),

    /// API status check
    Status,
}

pub async fn run(cmd: PmCmd, ctx: &Ctx) -> Result<(), HeatError> {
    match cmd.command {
        PmSubcommand::Price { token_id, side } => {
            clob::run(clob::ClobSubcommand::Price { token_id, side }, ctx).await
        }
        PmSubcommand::Buy {
            token_id,
            price,
            size,
            sig_type,
        } => {
            clob::run(
                clob::ClobSubcommand::LimitOrder {
                    token_id,
                    side: "buy".to_string(),
                    price,
                    size,
                    order_type: "gtc".to_string(),
                    post_only: false,
                    sig_type,
                },
                ctx,
            )
            .await
        }
        PmSubcommand::Sell {
            token_id,
            price,
            size,
            sig_type,
        } => {
            clob::run(
                clob::ClobSubcommand::LimitOrder {
                    token_id,
                    side: "sell".to_string(),
                    price,
                    size,
                    order_type: "gtc".to_string(),
                    post_only: false,
                    sig_type,
                },
                ctx,
            )
            .await
        }
        PmSubcommand::Cancel { order_id, sig_type } => {
            clob::run(
                clob::ClobSubcommand::CancelOrder { order_id, sig_type },
                ctx,
            )
            .await
        }
        PmSubcommand::Orders { market, sig_type } => {
            clob::run(
                clob::ClobSubcommand::Orders {
                    market,
                    asset_id: None,
                    sig_type,
                },
                ctx,
            )
            .await
        }
        PmSubcommand::Trades { market, sig_type } => {
            clob::run(
                clob::ClobSubcommand::Trades {
                    market,
                    asset_id: None,
                    sig_type,
                },
                ctx,
            )
            .await
        }
        PmSubcommand::Positions { user, limit } => {
            data::run(
                data::DataSubcommand::Positions {
                    user,
                    limit,
                    offset: None,
                },
                ctx,
            )
            .await
        }
        PmSubcommand::Balance { sig_type } => {
            clob::run(
                clob::ClobSubcommand::BalanceAllowance {
                    asset_type: "collateral".to_string(),
                    token_id: None,
                    sig_type,
                },
                ctx,
            )
            .await
        }
        PmSubcommand::Markets(sub) => gamma::markets(sub, ctx).await,
        PmSubcommand::Events(sub) => gamma::events(sub, ctx).await,
        PmSubcommand::Tags(sub) => gamma::tags(sub, ctx).await,
        PmSubcommand::Series(sub) => gamma::series(sub, ctx).await,
        PmSubcommand::Comments(sub) => gamma::comments(sub, ctx).await,
        PmSubcommand::Profiles(sub) => gamma::profiles(sub, ctx).await,
        PmSubcommand::Sports(sub) => gamma::sports(sub, ctx).await,
        PmSubcommand::Clob(sub) => clob::run(sub, ctx).await,
        PmSubcommand::Ctf(sub) => ctf::run(sub, ctx).await,
        PmSubcommand::Data(sub) => data::run(sub, ctx).await,
        PmSubcommand::Bridge(sub) => bridge::run(sub, ctx).await,
        PmSubcommand::Approve(sub) => approve::run(sub, ctx).await,
        PmSubcommand::Status => gamma::status(ctx).await,
    }
}
