/// DefiLlama command tree.
///
/// Read-only data commands against the DefiLlama API. All output uses
/// Heat-owned DTOs mapped from raw API responses.
use clap::{Args, Subcommand};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;

use crate::client::DefiLlamaClient;
use crate::config;
use crate::dto;
use crate::map;

// ---------------------------------------------------------------------------
// Top-level command
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct LlamaCmd {
    #[command(subcommand)]
    pub command: LlamaSubcommand,
}

#[derive(Subcommand)]
pub enum LlamaSubcommand {
    /// Protocol TVL data
    Protocols(ProtocolsCmd),
    /// Chain TVL data
    Chains(ChainsCmd),
    /// Token price data
    Coins(CoinsCmd),
    /// Stablecoin data
    Stablecoins(StablecoinsCmd),
    /// Bridge data
    Bridges(BridgesCmd),
    /// Protocol fees and revenue
    Fees(FeesCmd),
    /// DEX volume data
    Volumes(VolumesCmd),
    /// Yield and rate data
    Yields(YieldsCmd),
    /// API usage stats (requires pro key)
    Usage,
}

// ---------------------------------------------------------------------------
// Protocols
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct ProtocolsCmd {
    #[command(subcommand)]
    pub command: ProtocolsSubcommand,
}

#[derive(Subcommand)]
pub enum ProtocolsSubcommand {
    /// List all protocols with TVL
    List {
        /// Filter by category (e.g. Lending, DEX, Bridge)
        #[arg(long)]
        category: Option<String>,
        /// Filter by chain name (e.g. Ethereum, Arbitrum)
        #[arg(long)]
        chain: Option<String>,
        /// Sort by: tvl, change_1d, change_7d, name
        #[arg(long, default_value = "tvl")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Get detailed protocol data
    Get {
        /// Protocol slug (e.g. aave, uniswap)
        slug: String,
    },
    /// Get current TVL for a protocol
    Tvl {
        /// Protocol slug (e.g. aave, uniswap)
        slug: String,
    },
}

// ---------------------------------------------------------------------------
// Chains
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct ChainsCmd {
    #[command(subcommand)]
    pub command: ChainsSubcommand,
}

#[derive(Subcommand)]
pub enum ChainsSubcommand {
    /// List all chains with TVL
    List {
        /// Sort by: tvl, name
        #[arg(long, default_value = "tvl")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show historical TVL for all chains or a specific chain
    History {
        /// Chain name (e.g. Ethereum). Omit for aggregate TVL.
        chain: Option<String>,
        /// Show only last N data points
        #[arg(long)]
        limit: Option<usize>,
    },
}

// ---------------------------------------------------------------------------
// Coins
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct CoinsCmd {
    #[command(subcommand)]
    pub command: CoinsSubcommand,
}

#[derive(Subcommand)]
pub enum CoinsSubcommand {
    /// Get current token prices
    Price {
        /// Comma-separated coins in chain:address format (e.g. ethereum:0x...,coingecko:bitcoin)
        coins: String,
    },
    /// Get historical token prices at a specific timestamp
    Historical {
        /// Unix timestamp
        timestamp: i64,
        /// Comma-separated coins in chain:address format
        coins: String,
    },
    /// Get price chart for tokens
    Chart {
        /// Comma-separated coins in chain:address format
        coins: String,
        /// Time period (e.g. 1d, 7d, 30d, 90d, 1y, 5y)
        #[arg(long)]
        period: Option<String>,
        /// Number of data points
        #[arg(long)]
        span: Option<u32>,
    },
    /// Get percentage price change over a period
    Change {
        /// Comma-separated coins in chain:address format
        coins: String,
        /// Time period (e.g. 1d, 7d, 30d, 1y)
        #[arg(long, default_value = "1d")]
        period: String,
    },
    /// Get first recorded price for tokens
    First {
        /// Comma-separated coins in chain:address format
        coins: String,
    },
    /// Find block number closest to a timestamp
    Block {
        /// Chain name (e.g. ethereum, bsc, polygon)
        chain: String,
        /// Unix timestamp
        timestamp: i64,
    },
    /// Get historical liquidity for a token
    Liquidity {
        /// Token in chain:address format (e.g. ethereum:0x...)
        token: String,
        /// Show only last N data points
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Batch historical price lookup from JSON file
    BatchHistorical {
        /// Path to JSON file with batch request body
        file: String,
    },
}

// ---------------------------------------------------------------------------
// Stablecoins
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct StablecoinsCmd {
    #[command(subcommand)]
    pub command: StablecoinsSubcommand,
}

#[derive(Subcommand)]
pub enum StablecoinsSubcommand {
    /// List all stablecoins
    List {
        /// Sort by: circulating, name
        #[arg(long, default_value = "circulating")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Get detailed stablecoin data
    Get {
        /// Stablecoin ID (numeric)
        id: String,
    },
    /// List chains with stablecoin data
    Chains {
        /// Sort by: circulating, name
        #[arg(long, default_value = "circulating")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show historical stablecoin market cap
    Chart {
        /// Chain name (omit or use "all" for aggregate)
        chain: Option<String>,
        /// Show only last N data points
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Show stablecoin dominance for a chain
    Dominance {
        /// Chain name (e.g. Ethereum)
        chain: String,
        /// Show only last N data points
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Show current stablecoin prices
    Prices,
}

// ---------------------------------------------------------------------------
// Bridges
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct BridgesCmd {
    #[command(subcommand)]
    pub command: BridgesSubcommand,
}

#[derive(Subcommand)]
pub enum BridgesSubcommand {
    /// List all bridges
    List {
        /// Sort by: volume, name
        #[arg(long, default_value = "volume")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Get bridge details
    Get {
        /// Bridge ID (numeric)
        id: u64,
    },
    /// Show bridge volume for a chain
    Volume {
        /// Chain name (e.g. Ethereum)
        chain: String,
        /// Show only last N data points
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Show bridge day statistics
    Daystats {
        /// Unix timestamp for the day
        timestamp: i64,
        /// Chain name (e.g. Ethereum)
        chain: String,
    },
    /// Show bridge transactions
    Tx {
        /// Bridge ID (numeric)
        id: u64,
        /// Maximum number of transactions
        #[arg(long, default_value = "25")]
        limit: usize,
    },
}

// ---------------------------------------------------------------------------
// Fees
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct FeesCmd {
    #[command(subcommand)]
    pub command: FeesSubcommand,
}

#[derive(Subcommand)]
pub enum FeesSubcommand {
    /// Show fees overview across all protocols
    Overview {
        /// Filter by chain name (e.g. Ethereum)
        #[arg(long)]
        chain: Option<String>,
        /// Sort by: fees, change_1d, name
        #[arg(long, default_value = "fees")]
        sort: String,
        /// Maximum number of protocol rows
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show fees for a specific chain
    Chain {
        /// Chain name (e.g. Ethereum)
        chain: String,
        /// Sort by: fees, change_1d, name
        #[arg(long, default_value = "fees")]
        sort: String,
        /// Maximum number of protocol rows
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show fee summary for a specific protocol
    Protocol {
        /// Protocol slug (e.g. uniswap, aave)
        protocol: String,
    },
}

// ---------------------------------------------------------------------------
// Volumes
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct VolumesCmd {
    #[command(subcommand)]
    pub command: VolumesSubcommand,
}

#[derive(Subcommand)]
pub enum VolumesSubcommand {
    /// Show DEX volume overview
    Dexs {
        /// Filter by chain name (e.g. Ethereum)
        #[arg(long)]
        chain: Option<String>,
        /// Sort by: volume, change_1d, name
        #[arg(long, default_value = "volume")]
        sort: String,
        /// Maximum number of protocol rows
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show volume summary for a specific DEX
    Dex {
        /// Protocol slug
        protocol: String,
    },
    /// Show options volume overview
    Options {
        /// Filter by chain
        #[arg(long)]
        chain: Option<String>,
        /// Sort by: volume, change_1d, name
        #[arg(long, default_value = "volume")]
        sort: String,
        /// Maximum number of protocol rows
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show options volume for a specific protocol
    Option {
        /// Protocol slug
        protocol: String,
    },
    /// Show derivatives volume overview
    Derivatives {
        /// Filter by chain
        #[arg(long)]
        chain: Option<String>,
        /// Sort by: volume, change_1d, name
        #[arg(long, default_value = "volume")]
        sort: String,
        /// Maximum number of protocol rows
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show derivatives volume for a specific protocol
    Derivative {
        /// Protocol slug
        protocol: String,
    },
    /// Show open interest overview
    OpenInterest {
        /// Sort by: volume, change_1d, name
        #[arg(long, default_value = "volume")]
        sort: String,
        /// Maximum number of protocol rows
        #[arg(long, default_value = "25")]
        limit: usize,
    },
}

// ---------------------------------------------------------------------------
// Yields
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct YieldsCmd {
    #[command(subcommand)]
    pub command: YieldsSubcommand,
}

#[derive(Subcommand)]
pub enum YieldsSubcommand {
    /// List all yield pools
    Pools {
        /// Filter by chain (e.g. Ethereum)
        #[arg(long)]
        chain: Option<String>,
        /// Filter by project (e.g. aave-v3)
        #[arg(long)]
        project: Option<String>,
        /// Sort by: apy, tvl, name
        #[arg(long, default_value = "apy")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
        /// Show only stablecoin pools
        #[arg(long)]
        stablecoin: bool,
    },
    /// List pools with historical APY data
    PoolsOld {
        /// Filter by chain
        #[arg(long)]
        chain: Option<String>,
        /// Sort by: apy, tvl, name
        #[arg(long, default_value = "apy")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show APY/TVL chart for a pool
    Chart {
        /// Pool ID
        pool: String,
        /// Show only last N data points
        #[arg(long)]
        limit: Option<usize>,
    },
    /// List borrow/lend pools
    Borrow {
        /// Filter by chain
        #[arg(long)]
        chain: Option<String>,
        /// Filter by project
        #[arg(long)]
        project: Option<String>,
        /// Sort by: apy, borrow_apy, tvl, name
        #[arg(long, default_value = "tvl")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show lending/borrowing chart for a pool
    LendBorrowChart {
        /// Pool ID
        pool: String,
        /// Show only last N data points
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Show perpetual funding rates
    Perps {
        /// Filter by marketplace
        #[arg(long)]
        marketplace: Option<String>,
        /// Sort by: funding_rate, open_interest, name
        #[arg(long, default_value = "open_interest")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
    },
    /// Show liquid staking rates
    Lsd {
        /// Sort by: apy, market_share, name
        #[arg(long, default_value = "market_share")]
        sort: String,
        /// Maximum number of results
        #[arg(long, default_value = "25")]
        limit: usize,
    },
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub async fn run(cmd: LlamaCmd, ctx: &Ctx) -> Result<(), HeatError> {
    match cmd.command {
        LlamaSubcommand::Protocols(c) => match c.command {
            ProtocolsSubcommand::List {
                category,
                chain,
                sort,
                limit,
            } => cmd_protocols_list(category, chain, &sort, limit, ctx).await,
            ProtocolsSubcommand::Get { slug } => cmd_protocols_get(&slug, ctx).await,
            ProtocolsSubcommand::Tvl { slug } => cmd_protocols_tvl(&slug, ctx).await,
        },
        LlamaSubcommand::Chains(c) => match c.command {
            ChainsSubcommand::List { sort, limit } => cmd_chains_list(&sort, limit, ctx).await,
            ChainsSubcommand::History { chain, limit } => {
                cmd_chains_history(chain.as_deref(), limit, ctx).await
            }
        },
        LlamaSubcommand::Coins(c) => match c.command {
            CoinsSubcommand::Price { coins } => cmd_coins_price(&coins, ctx).await,
            CoinsSubcommand::Historical { timestamp, coins } => {
                cmd_coins_historical(timestamp, &coins, ctx).await
            }
            CoinsSubcommand::Chart {
                coins,
                period,
                span,
            } => cmd_coins_chart(&coins, period.as_deref(), span, ctx).await,
            CoinsSubcommand::Change { coins, period } => {
                cmd_coins_change(&coins, &period, ctx).await
            }
            CoinsSubcommand::First { coins } => cmd_coins_first(&coins, ctx).await,
            CoinsSubcommand::Block { chain, timestamp } => {
                cmd_coins_block(&chain, timestamp, ctx).await
            }
            CoinsSubcommand::Liquidity { token, limit } => {
                cmd_coins_liquidity(&token, limit, ctx).await
            }
            CoinsSubcommand::BatchHistorical { file } => {
                cmd_coins_batch_historical(&file, ctx).await
            }
        },
        LlamaSubcommand::Stablecoins(c) => match c.command {
            StablecoinsSubcommand::List { sort, limit } => {
                cmd_stablecoins_list(&sort, limit, ctx).await
            }
            StablecoinsSubcommand::Get { id } => cmd_stablecoins_get(&id, ctx).await,
            StablecoinsSubcommand::Chains { sort, limit } => {
                cmd_stablecoins_chains(&sort, limit, ctx).await
            }
            StablecoinsSubcommand::Chart { chain, limit } => {
                cmd_stablecoins_chart(chain.as_deref(), limit, ctx).await
            }
            StablecoinsSubcommand::Dominance { chain, limit } => {
                cmd_stablecoins_dominance(&chain, limit, ctx).await
            }
            StablecoinsSubcommand::Prices => cmd_stablecoins_prices(ctx).await,
        },
        LlamaSubcommand::Bridges(c) => match c.command {
            BridgesSubcommand::List { sort, limit } => cmd_bridges_list(&sort, limit, ctx).await,
            BridgesSubcommand::Get { id } => cmd_bridges_get(id, ctx).await,
            BridgesSubcommand::Volume { chain, limit } => {
                cmd_bridges_volume(&chain, limit, ctx).await
            }
            BridgesSubcommand::Daystats { timestamp, chain } => {
                cmd_bridges_daystats(timestamp, &chain, ctx).await
            }
            BridgesSubcommand::Tx { id, limit } => cmd_bridges_tx(id, limit, ctx).await,
        },
        LlamaSubcommand::Fees(c) => match c.command {
            FeesSubcommand::Overview { chain, sort, limit } => {
                cmd_fees_overview(chain.as_deref(), &sort, limit, ctx).await
            }
            FeesSubcommand::Chain { chain, sort, limit } => {
                cmd_fees_chain(&chain, &sort, limit, ctx).await
            }
            FeesSubcommand::Protocol { protocol } => cmd_fees_protocol(&protocol, ctx).await,
        },
        LlamaSubcommand::Volumes(c) => match c.command {
            VolumesSubcommand::Dexs { chain, sort, limit } => {
                cmd_volumes_dexs(chain.as_deref(), &sort, limit, ctx).await
            }
            VolumesSubcommand::Dex { protocol } => cmd_volumes_dex_summary(&protocol, ctx).await,
            VolumesSubcommand::Options { chain, sort, limit } => {
                cmd_volumes_options(chain.as_deref(), &sort, limit, ctx).await
            }
            VolumesSubcommand::Option { protocol } => {
                cmd_volumes_option_summary(&protocol, ctx).await
            }
            VolumesSubcommand::Derivatives { chain, sort, limit } => {
                cmd_volumes_derivatives(chain.as_deref(), &sort, limit, ctx).await
            }
            VolumesSubcommand::Derivative { protocol } => {
                cmd_volumes_derivative_summary(&protocol, ctx).await
            }
            VolumesSubcommand::OpenInterest { sort, limit } => {
                cmd_volumes_open_interest(&sort, limit, ctx).await
            }
        },
        LlamaSubcommand::Yields(c) => match c.command {
            YieldsSubcommand::Pools {
                chain,
                project,
                sort,
                limit,
                stablecoin,
            } => {
                cmd_yields_pools(
                    chain.as_deref(),
                    project.as_deref(),
                    &sort,
                    limit,
                    stablecoin,
                    ctx,
                )
                .await
            }
            YieldsSubcommand::PoolsOld { chain, sort, limit } => {
                cmd_yields_pools_old(chain.as_deref(), &sort, limit, ctx).await
            }
            YieldsSubcommand::Chart { pool, limit } => cmd_yields_chart(&pool, limit, ctx).await,
            YieldsSubcommand::Borrow {
                chain,
                project,
                sort,
                limit,
            } => cmd_yields_borrow(chain.as_deref(), project.as_deref(), &sort, limit, ctx).await,
            YieldsSubcommand::LendBorrowChart { pool, limit } => {
                cmd_yields_lend_borrow_chart(&pool, limit, ctx).await
            }
            YieldsSubcommand::Perps {
                marketplace,
                sort,
                limit,
            } => cmd_yields_perps(marketplace.as_deref(), &sort, limit, ctx).await,
            YieldsSubcommand::Lsd { sort, limit } => cmd_yields_lsd(&sort, limit, ctx).await,
        },
        LlamaSubcommand::Usage => cmd_usage(ctx).await,
    }
}

// ---------------------------------------------------------------------------
// Command implementations
// ---------------------------------------------------------------------------

fn make_client(ctx: &Ctx) -> Result<DefiLlamaClient, HeatError> {
    let api_key = config::resolve_api_key(&ctx.config);
    DefiLlamaClient::new(api_key)
}

async fn cmd_protocols_list(
    category: Option<String>,
    chain: Option<String>,
    sort: &str,
    limit: usize,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.protocols().await?;
    let mut dto = map::map_protocols(raw);

    // Filter
    if let Some(cat) = &category {
        let cat_lower = cat.to_lowercase();
        dto.protocols.retain(|p| {
            p.category
                .as_ref()
                .is_some_and(|c| c.to_lowercase() == cat_lower)
        });
    }
    if let Some(ch) = &chain {
        let ch_lower = ch.to_lowercase();
        dto.protocols
            .retain(|p| p.chains.iter().any(|c| c.to_lowercase() == ch_lower));
    }

    // Sort
    validate_sort(sort, PROTOCOL_SORT_VALUES)?;
    sort_protocols(&mut dto.protocols, sort);

    // Limit
    dto.protocols.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_protocols_list))
        .map_err(io_err)
}

async fn cmd_protocols_get(slug: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.protocol(slug).await?;
    let dto = map::map_protocol_detail(raw, slug);

    ctx.output
        .write_data(&dto, Some(&pretty_protocol_detail))
        .map_err(io_err)
}

async fn cmd_protocols_tvl(slug: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let tvl = client.tvl(slug).await?;
    let dto = dto::ProtocolTvlDto {
        slug: slug.to_owned(),
        tvl_usd: tvl,
    };

    if ctx.output.format == OutputFormat::Quiet {
        return ctx
            .output
            .write_scalar(&format!("{:.2}", tvl))
            .map_err(io_err);
    }

    ctx.output
        .write_data(&dto, Some(&pretty_protocol_tvl))
        .map_err(io_err)
}

async fn cmd_chains_list(sort: &str, limit: usize, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.chains().await?;
    let mut dto = map::map_chains(raw);

    validate_sort(sort, CHAIN_SORT_VALUES)?;
    sort_chains(&mut dto.chains, sort);
    dto.chains.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_chains_list))
        .map_err(io_err)
}

async fn cmd_chains_history(
    chain: Option<&str>,
    limit: Option<usize>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.historical_chain_tvl(chain).await?;
    let mut dto = map::map_chain_history(raw, chain);

    if let Some(n) = limit {
        let len = dto.points.len();
        if n < len {
            dto.points = dto.points.split_off(len - n);
        }
    }

    ctx.output
        .write_data(&dto, Some(&pretty_chain_history))
        .map_err(io_err)
}

async fn cmd_coins_price(coins: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.coins_price(coins).await?;
    let dto = map::map_coins_price(raw);

    ctx.output
        .write_data(&dto, Some(&pretty_coins_price))
        .map_err(io_err)
}

async fn cmd_coins_historical(timestamp: i64, coins: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.coins_historical(timestamp, coins).await?;
    let dto = map::map_coins_price(raw);

    ctx.output
        .write_data(&dto, Some(&pretty_coins_price))
        .map_err(io_err)
}

async fn cmd_coins_chart(
    coins: &str,
    period: Option<&str>,
    span: Option<u32>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.coins_chart(coins, period, span).await?;
    let dto = map::map_coins_chart(raw);

    ctx.output
        .write_data(&dto, Some(&pretty_coins_chart))
        .map_err(io_err)
}

async fn cmd_coins_change(coins: &str, period: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.coins_change(coins, Some(period)).await?;
    let dto = map::map_coins_change(raw, Some(period));
    ctx.output
        .write_data(&dto, Some(&pretty_coins_change))
        .map_err(io_err)
}

async fn cmd_coins_first(coins: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.coins_first(coins).await?;
    let dto = map::map_coins_price(raw);
    ctx.output
        .write_data(&dto, Some(&pretty_coins_price))
        .map_err(io_err)
}

async fn cmd_coins_block(chain: &str, timestamp: i64, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.coins_block(chain, timestamp).await?;
    let dto = map::map_block(raw, chain);
    ctx.output
        .write_data(&dto, Some(&pretty_block))
        .map_err(io_err)
}

async fn cmd_coins_liquidity(
    token: &str,
    limit: Option<usize>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    config::require_api_key(&ctx.config)?;
    let client = make_client(ctx)?;
    let raw = client.coins_liquidity(token).await?;
    let mut dto = map::map_coins_liquidity(raw, token);
    if let Some(n) = limit {
        let len = dto.points.len();
        if n < len {
            dto.points = dto.points.split_off(len - n);
        }
    }
    ctx.output
        .write_data(&dto, Some(&pretty_coins_liquidity))
        .map_err(io_err)
}

async fn cmd_coins_batch_historical(file: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| HeatError::validation("file_read", format!("Failed to read {file}: {e}")))?;
    let body: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| HeatError::validation("file_parse", format!("Invalid JSON in {file}: {e}")))?;
    let client = make_client(ctx)?;
    let raw = client.coins_batch_historical(&body).await?;
    let dto = map::map_coins_price(raw);
    ctx.output
        .write_data(&dto, Some(&pretty_coins_price))
        .map_err(io_err)
}

async fn cmd_stablecoins_list(sort: &str, limit: usize, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.stablecoins().await?;
    let mut dto = map::map_stablecoins(raw);

    validate_sort(sort, STABLECOIN_SORT_VALUES)?;
    sort_stablecoins(&mut dto.stablecoins, sort);
    dto.stablecoins.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_stablecoins_list))
        .map_err(io_err)
}

async fn cmd_stablecoins_get(id: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.stablecoin(id).await?;
    let dto = map::map_stablecoin_detail(raw);

    ctx.output
        .write_data(&dto, Some(&pretty_stablecoin_detail))
        .map_err(io_err)
}

async fn cmd_stablecoins_chains(sort: &str, limit: usize, ctx: &Ctx) -> Result<(), HeatError> {
    validate_sort(sort, STABLECOIN_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.stablecoin_chains().await?;
    let mut dto = map::map_stablecoin_chains(raw);
    match sort {
        "name" => dto
            .chains
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        _ => dto.chains.sort_by(|a, b| {
            b.circulating_usd
                .unwrap_or(0.0)
                .partial_cmp(&a.circulating_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
    dto.chains.truncate(limit);
    ctx.output
        .write_data(&dto, Some(&pretty_stablecoin_chains))
        .map_err(io_err)
}

async fn cmd_stablecoins_chart(
    chain: Option<&str>,
    limit: Option<usize>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let effective_chain = match chain {
        Some("all") | None => None,
        Some(c) => Some(c),
    };
    let raw = client.stablecoin_charts(effective_chain).await?;
    let mut dto = map::map_stablecoin_chart(raw, effective_chain);
    if let Some(n) = limit {
        let len = dto.points.len();
        if n < len {
            dto.points = dto.points.split_off(len - n);
        }
    }
    ctx.output
        .write_data(&dto, Some(&pretty_stablecoin_chart))
        .map_err(io_err)
}

async fn cmd_stablecoins_dominance(
    chain: &str,
    limit: Option<usize>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    config::require_api_key(&ctx.config)?;
    let client = make_client(ctx)?;
    let raw = client.stablecoin_dominance(chain).await?;
    let mut dto = map::map_stablecoin_dominance(raw, chain);
    if let Some(n) = limit {
        let len = dto.points.len();
        if n < len {
            dto.points = dto.points.split_off(len - n);
        }
    }
    ctx.output
        .write_data(&dto, Some(&pretty_stablecoin_dominance))
        .map_err(io_err)
}

async fn cmd_stablecoins_prices(ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.stablecoin_prices().await?;
    let dto = map::map_stablecoin_prices(raw);
    ctx.output
        .write_data(&dto, Some(&pretty_stablecoin_prices))
        .map_err(io_err)
}

async fn cmd_bridges_list(sort: &str, limit: usize, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.bridges().await?;
    let mut dto = map::map_bridges(raw);

    validate_sort(sort, BRIDGE_SORT_VALUES)?;
    sort_bridges(&mut dto.bridges, sort);
    dto.bridges.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_bridges_list))
        .map_err(io_err)
}

async fn cmd_bridges_get(id: u64, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.bridge(id).await?;
    let dto = map::map_bridge_detail(raw);

    ctx.output
        .write_data(&dto, Some(&pretty_bridge_detail))
        .map_err(io_err)
}

async fn cmd_bridges_volume(chain: &str, limit: Option<usize>, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.bridge_volume(chain).await?;
    let mut dto = map::map_bridge_volume(raw, chain);
    if let Some(n) = limit {
        let len = dto.points.len();
        if n < len {
            dto.points = dto.points.split_off(len - n);
        }
    }
    ctx.output
        .write_data(&dto, Some(&pretty_bridge_volume))
        .map_err(io_err)
}

async fn cmd_bridges_daystats(timestamp: i64, chain: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.bridge_daystats(timestamp, chain).await?;
    let dto = map::map_bridge_daystats(raw, chain, timestamp);
    ctx.output
        .write_data(&dto, Some(&pretty_bridge_daystats))
        .map_err(io_err)
}

async fn cmd_bridges_tx(id: u64, limit: usize, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.bridge_transactions(id).await?;
    let mut dto = map::map_bridge_transactions(raw, id);
    dto.transactions.truncate(limit);
    ctx.output
        .write_data(&dto, Some(&pretty_bridge_tx))
        .map_err(io_err)
}

async fn cmd_fees_overview(
    chain: Option<&str>,
    sort: &str,
    limit: usize,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.fees_overview(chain).await?;
    let mut dto = map::map_overview(raw, "fees", chain);

    validate_sort(sort, OVERVIEW_SORT_VALUES)?;
    sort_overview(&mut dto.protocols, sort);
    dto.protocols.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_overview))
        .map_err(io_err)
}

async fn cmd_volumes_dexs(
    chain: Option<&str>,
    sort: &str,
    limit: usize,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.volumes_dexs(chain).await?;
    let mut dto = map::map_overview(raw, "dex_volume", chain);

    validate_sort(sort, OVERVIEW_SORT_VALUES)?;
    sort_overview(&mut dto.protocols, sort);
    dto.protocols.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_overview))
        .map_err(io_err)
}

async fn cmd_fees_chain(chain: &str, sort: &str, limit: usize, ctx: &Ctx) -> Result<(), HeatError> {
    validate_sort(sort, OVERVIEW_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.fees_overview(Some(chain)).await?;
    let mut dto = map::map_overview(raw, "fees", Some(chain));
    sort_overview(&mut dto.protocols, sort);
    dto.protocols.truncate(limit);
    ctx.output
        .write_data(&dto, Some(&pretty_overview))
        .map_err(io_err)
}

async fn cmd_fees_protocol(protocol: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.fees_protocol(protocol).await?;
    let dto = map::map_protocol_summary(raw, "fees");
    ctx.output
        .write_data(&dto, Some(&pretty_protocol_summary))
        .map_err(io_err)
}

async fn cmd_volumes_dex_summary(protocol: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.volumes_dex_summary(protocol).await?;
    let dto = map::map_protocol_summary(raw, "dex_volume");
    ctx.output
        .write_data(&dto, Some(&pretty_protocol_summary))
        .map_err(io_err)
}

async fn cmd_volumes_options(
    chain: Option<&str>,
    sort: &str,
    limit: usize,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    validate_sort(sort, OVERVIEW_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.volumes_options(chain).await?;
    let mut dto = map::map_overview(raw, "options_volume", chain);
    sort_overview(&mut dto.protocols, sort);
    dto.protocols.truncate(limit);
    ctx.output
        .write_data(&dto, Some(&pretty_overview))
        .map_err(io_err)
}

async fn cmd_volumes_option_summary(protocol: &str, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.volumes_option_summary(protocol).await?;
    let dto = map::map_protocol_summary(raw, "options_volume");
    ctx.output
        .write_data(&dto, Some(&pretty_protocol_summary))
        .map_err(io_err)
}

async fn cmd_volumes_derivatives(
    chain: Option<&str>,
    sort: &str,
    limit: usize,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    config::require_api_key(&ctx.config)?;
    validate_sort(sort, OVERVIEW_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.volumes_derivatives(chain).await?;
    let mut dto = map::map_overview(raw, "derivatives_volume", chain);
    sort_overview(&mut dto.protocols, sort);
    dto.protocols.truncate(limit);
    ctx.output
        .write_data(&dto, Some(&pretty_overview))
        .map_err(io_err)
}

async fn cmd_volumes_derivative_summary(protocol: &str, ctx: &Ctx) -> Result<(), HeatError> {
    config::require_api_key(&ctx.config)?;
    let client = make_client(ctx)?;
    let raw = client.volumes_derivative_summary(protocol).await?;
    let dto = map::map_protocol_summary(raw, "derivatives_volume");
    ctx.output
        .write_data(&dto, Some(&pretty_protocol_summary))
        .map_err(io_err)
}

async fn cmd_volumes_open_interest(sort: &str, limit: usize, ctx: &Ctx) -> Result<(), HeatError> {
    validate_sort(sort, OVERVIEW_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.volumes_open_interest().await?;
    let mut dto = map::map_overview(raw, "open_interest", None);
    sort_overview(&mut dto.protocols, sort);
    dto.protocols.truncate(limit);
    ctx.output
        .write_data(&dto, Some(&pretty_overview))
        .map_err(io_err)
}

// ---------------------------------------------------------------------------
// Yields command implementations
// ---------------------------------------------------------------------------

async fn cmd_yields_pools(
    chain: Option<&str>,
    project: Option<&str>,
    sort: &str,
    limit: usize,
    stablecoin_only: bool,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    validate_sort(sort, YIELD_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.yields_pools().await?;
    let mut dto = map::map_yield_pools(raw);

    if let Some(ch) = chain {
        let ch_lower = ch.to_lowercase();
        dto.pools.retain(|p| {
            p.chain
                .as_ref()
                .is_some_and(|c| c.to_lowercase() == ch_lower)
        });
    }
    if let Some(proj) = project {
        let proj_lower = proj.to_lowercase();
        dto.pools.retain(|p| {
            p.project
                .as_ref()
                .is_some_and(|pr| pr.to_lowercase() == proj_lower)
        });
    }
    if stablecoin_only {
        dto.pools.retain(|p| p.stablecoin == Some(true));
    }

    sort_yield_pools(&mut dto.pools, sort);
    dto.pools.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_yield_pools))
        .map_err(io_err)
}

async fn cmd_yields_pools_old(
    chain: Option<&str>,
    sort: &str,
    limit: usize,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    config::require_api_key(&ctx.config)?;
    validate_sort(sort, YIELD_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.yields_pools_old().await?;
    let mut dto = map::map_yield_pools(raw);

    if let Some(ch) = chain {
        let ch_lower = ch.to_lowercase();
        dto.pools.retain(|p| {
            p.chain
                .as_ref()
                .is_some_and(|c| c.to_lowercase() == ch_lower)
        });
    }

    sort_yield_pools(&mut dto.pools, sort);
    dto.pools.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_yield_pools))
        .map_err(io_err)
}

async fn cmd_yields_chart(pool: &str, limit: Option<usize>, ctx: &Ctx) -> Result<(), HeatError> {
    let client = make_client(ctx)?;
    let raw = client.yields_chart(pool).await?;
    let mut dto = map::map_yield_chart(raw, pool);
    if let Some(n) = limit {
        let len = dto.points.len();
        if n < len {
            dto.points = dto.points.split_off(len - n);
        }
    }
    ctx.output
        .write_data(&dto, Some(&pretty_yield_chart))
        .map_err(io_err)
}

async fn cmd_yields_borrow(
    chain: Option<&str>,
    project: Option<&str>,
    sort: &str,
    limit: usize,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    config::require_api_key(&ctx.config)?;
    validate_sort(sort, YIELD_BORROW_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.yields_borrow().await?;
    let mut dto = map::map_yield_borrow_pools(raw);

    if let Some(ch) = chain {
        let ch_lower = ch.to_lowercase();
        dto.pools.retain(|p| {
            p.chain
                .as_ref()
                .is_some_and(|c| c.to_lowercase() == ch_lower)
        });
    }
    if let Some(proj) = project {
        let proj_lower = proj.to_lowercase();
        dto.pools.retain(|p| {
            p.project
                .as_ref()
                .is_some_and(|pr| pr.to_lowercase() == proj_lower)
        });
    }

    sort_yield_borrow_pools(&mut dto.pools, sort);
    dto.pools.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_yield_borrow_pools))
        .map_err(io_err)
}

async fn cmd_yields_lend_borrow_chart(
    pool: &str,
    limit: Option<usize>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    config::require_api_key(&ctx.config)?;
    let client = make_client(ctx)?;
    let raw = client.yields_lend_borrow_chart(pool).await?;
    let mut dto = map::map_yield_lend_borrow_chart(raw, pool);
    if let Some(n) = limit {
        let len = dto.points.len();
        if n < len {
            dto.points = dto.points.split_off(len - n);
        }
    }
    ctx.output
        .write_data(&dto, Some(&pretty_yield_lend_borrow_chart))
        .map_err(io_err)
}

async fn cmd_yields_perps(
    marketplace: Option<&str>,
    sort: &str,
    limit: usize,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    config::require_api_key(&ctx.config)?;
    validate_sort(sort, PERP_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.yields_perps().await?;
    let mut dto = map::map_perps(raw);

    if let Some(m) = marketplace {
        let m_lower = m.to_lowercase();
        dto.perps.retain(|p| {
            p.marketplace
                .as_ref()
                .is_some_and(|mp| mp.to_lowercase() == m_lower)
        });
    }

    sort_perps(&mut dto.perps, sort);
    dto.perps.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_perps))
        .map_err(io_err)
}

async fn cmd_yields_lsd(sort: &str, limit: usize, ctx: &Ctx) -> Result<(), HeatError> {
    config::require_api_key(&ctx.config)?;
    validate_sort(sort, LSD_SORT_VALUES)?;
    let client = make_client(ctx)?;
    let raw = client.yields_lsd().await?;
    let mut dto = map::map_lsd(raw);

    sort_lsd(&mut dto.rates, sort);
    dto.rates.truncate(limit);

    ctx.output
        .write_data(&dto, Some(&pretty_lsd))
        .map_err(io_err)
}

// ---------------------------------------------------------------------------
// Sorting helpers
// ---------------------------------------------------------------------------

fn validate_sort(sort: &str, valid: &[&str]) -> Result<(), HeatError> {
    if valid.contains(&sort) {
        Ok(())
    } else {
        Err(HeatError::validation(
            "invalid_sort",
            format!(
                "Unknown sort value '{}'. Valid options: {}",
                sort,
                valid.join(", ")
            ),
        ))
    }
}

const PROTOCOL_SORT_VALUES: &[&str] = &["tvl", "name", "change_1d", "change_7d"];
const CHAIN_SORT_VALUES: &[&str] = &["tvl", "name"];
const STABLECOIN_SORT_VALUES: &[&str] = &["circulating", "name"];
const BRIDGE_SORT_VALUES: &[&str] = &["volume", "name"];
const OVERVIEW_SORT_VALUES: &[&str] = &["fees", "volume", "name", "change_1d"];
const YIELD_SORT_VALUES: &[&str] = &["apy", "tvl", "name"];
const YIELD_BORROW_SORT_VALUES: &[&str] = &["apy", "borrow_apy", "tvl", "name"];
const PERP_SORT_VALUES: &[&str] = &["funding_rate", "open_interest", "name"];
const LSD_SORT_VALUES: &[&str] = &["apy", "market_share", "name"];

fn sort_protocols(protocols: &mut [dto::ProtocolRow], sort: &str) {
    match sort {
        "name" => protocols.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        "change_1d" => protocols.sort_by(|a, b| {
            b.change_1d_pct
                .unwrap_or(f64::NEG_INFINITY)
                .partial_cmp(&a.change_1d_pct.unwrap_or(f64::NEG_INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "change_7d" => protocols.sort_by(|a, b| {
            b.change_7d_pct
                .unwrap_or(f64::NEG_INFINITY)
                .partial_cmp(&a.change_7d_pct.unwrap_or(f64::NEG_INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => protocols.sort_by(|a, b| {
            b.tvl_usd
                .unwrap_or(0.0)
                .partial_cmp(&a.tvl_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

fn sort_chains(chains: &mut [dto::ChainRow], sort: &str) {
    match sort {
        "name" => chains.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        _ => chains.sort_by(|a, b| {
            b.tvl_usd
                .unwrap_or(0.0)
                .partial_cmp(&a.tvl_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

fn sort_stablecoins(stablecoins: &mut [dto::StablecoinRow], sort: &str) {
    match sort {
        "name" => {
            stablecoins.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
        _ => stablecoins.sort_by(|a, b| {
            b.circulating_usd
                .unwrap_or(0.0)
                .partial_cmp(&a.circulating_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

fn sort_bridges(bridges: &mut [dto::BridgeRow], sort: &str) {
    match sort {
        "name" => bridges.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        _ => bridges.sort_by(|a, b| {
            b.daily_volume_usd
                .unwrap_or(0.0)
                .partial_cmp(&a.daily_volume_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

fn sort_overview(protocols: &mut [dto::OverviewProtocolRow], sort: &str) {
    match sort {
        "name" => protocols.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        "change_1d" => protocols.sort_by(|a, b| {
            b.change_1d_pct
                .unwrap_or(f64::NEG_INFINITY)
                .partial_cmp(&a.change_1d_pct.unwrap_or(f64::NEG_INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => protocols.sort_by(|a, b| {
            b.total_24h_usd
                .unwrap_or(0.0)
                .partial_cmp(&a.total_24h_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

fn sort_yield_pools(pools: &mut [dto::YieldPoolRow], sort: &str) {
    match sort {
        "name" => pools.sort_by(|a, b| {
            let a_name = a.project.as_deref().unwrap_or("");
            let b_name = b.project.as_deref().unwrap_or("");
            a_name.to_lowercase().cmp(&b_name.to_lowercase())
        }),
        "tvl" => pools.sort_by(|a, b| {
            b.tvl_usd
                .unwrap_or(0.0)
                .partial_cmp(&a.tvl_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => pools.sort_by(|a, b| {
            b.apy
                .unwrap_or(0.0)
                .partial_cmp(&a.apy.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

fn sort_yield_borrow_pools(pools: &mut [dto::YieldBorrowPoolRow], sort: &str) {
    match sort {
        "name" => pools.sort_by(|a, b| {
            let a_name = a.project.as_deref().unwrap_or("");
            let b_name = b.project.as_deref().unwrap_or("");
            a_name.to_lowercase().cmp(&b_name.to_lowercase())
        }),
        "apy" => pools.sort_by(|a, b| {
            b.apy
                .unwrap_or(0.0)
                .partial_cmp(&a.apy.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "borrow_apy" => pools.sort_by(|a, b| {
            let a_borrow = a.apy_base_borrow.unwrap_or(0.0) + a.apy_reward_borrow.unwrap_or(0.0);
            let b_borrow = b.apy_base_borrow.unwrap_or(0.0) + b.apy_reward_borrow.unwrap_or(0.0);
            b_borrow
                .partial_cmp(&a_borrow)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => pools.sort_by(|a, b| {
            b.tvl_usd
                .unwrap_or(0.0)
                .partial_cmp(&a.tvl_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

fn sort_perps(perps: &mut [dto::PerpRow], sort: &str) {
    match sort {
        "name" => perps.sort_by(|a, b| {
            let a_s = a.symbol.as_deref().unwrap_or("");
            let b_s = b.symbol.as_deref().unwrap_or("");
            a_s.to_lowercase().cmp(&b_s.to_lowercase())
        }),
        "funding_rate" => perps.sort_by(|a, b| {
            b.funding_rate
                .unwrap_or(0.0)
                .abs()
                .partial_cmp(&a.funding_rate.unwrap_or(0.0).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => perps.sort_by(|a, b| {
            b.open_interest
                .unwrap_or(0.0)
                .partial_cmp(&a.open_interest.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

// ---------------------------------------------------------------------------
// Ecosystem command implementations
// ---------------------------------------------------------------------------

fn sort_lsd(rates: &mut [dto::LsdRow], sort: &str) {
    match sort {
        "name" => rates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        "apy" => rates.sort_by(|a, b| {
            b.apy
                .unwrap_or(0.0)
                .partial_cmp(&a.apy.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => rates.sort_by(|a, b| {
            b.market_share
                .unwrap_or(0.0)
                .partial_cmp(&a.market_share.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

// ---------------------------------------------------------------------------
// Pretty formatters
// ---------------------------------------------------------------------------

fn fmt_usd(val: Option<f64>) -> String {
    match val {
        Some(v) if v >= 1_000_000_000.0 => format!("${:.2}B", v / 1_000_000_000.0),
        Some(v) if v >= 1_000_000.0 => format!("${:.2}M", v / 1_000_000.0),
        Some(v) if v >= 1_000.0 => format!("${:.2}K", v / 1_000.0),
        Some(v) => format!("${:.2}", v),
        None => "-".to_owned(),
    }
}

fn fmt_pct(val: Option<f64>) -> String {
    match val {
        Some(v) => format!("{:+.2}%", v),
        None => "-".to_owned(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_owned()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

fn pretty_protocols_list(dto: &dto::ProtocolsListDto) -> String {
    let mut out = format!(
        "{:<25} {:<10} {:>12} {:>8} {:>8}\n",
        "NAME", "CATEGORY", "TVL", "1D", "7D"
    );
    out.push_str(&"-".repeat(67));
    out.push('\n');
    for p in &dto.protocols {
        out.push_str(&format!(
            "{:<25} {:<10} {:>12} {:>8} {:>8}\n",
            truncate(&p.name, 24),
            truncate(p.category.as_deref().unwrap_or("-"), 9),
            fmt_usd(p.tvl_usd),
            fmt_pct(p.change_1d_pct),
            fmt_pct(p.change_7d_pct),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_protocol_detail(dto: &dto::ProtocolDetailDto) -> String {
    let mut out = format!("{} ({})\n", dto.name, dto.slug);
    if let Some(sym) = &dto.symbol {
        out.push_str(&format!("Symbol:      {sym}\n"));
    }
    if let Some(cat) = &dto.category {
        out.push_str(&format!("Category:    {cat}\n"));
    }
    if let Some(desc) = &dto.description {
        out.push_str(&format!("Description: {}\n", truncate(desc, 80)));
    }
    if let Some(url) = &dto.url {
        out.push_str(&format!("URL:         {url}\n"));
    }
    out.push_str(&format!("TVL:         {}\n", fmt_usd(dto.tvl_usd)));
    if let Some(mcap) = dto.mcap_usd {
        out.push_str(&format!("Market cap:  {}\n", fmt_usd(Some(mcap))));
    }
    if !dto.chains.is_empty() {
        out.push_str(&format!("Chains:      {}\n", dto.chains.join(", ")));
    }
    if !dto.chain_tvls.is_empty() {
        out.push_str("Chain TVLs:\n");
        let mut sorted: Vec<_> = dto.chain_tvls.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (chain, tvl) in sorted {
            out.push_str(&format!("  {:<20} {}\n", chain, fmt_usd(Some(*tvl))));
        }
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_protocol_tvl(dto: &dto::ProtocolTvlDto) -> String {
    format!("{}: {}", dto.slug, fmt_usd(Some(dto.tvl_usd)))
}

fn pretty_chains_list(dto: &dto::ChainsListDto) -> String {
    let mut out = format!("{:<25} {:<8} {:>14}\n", "CHAIN", "TOKEN", "TVL");
    out.push_str(&"-".repeat(50));
    out.push('\n');
    for c in &dto.chains {
        out.push_str(&format!(
            "{:<25} {:<8} {:>14}\n",
            truncate(&c.name, 24),
            c.token_symbol.as_deref().unwrap_or("-"),
            fmt_usd(c.tvl_usd),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_chain_history(dto: &dto::ChainHistoryDto) -> String {
    let label = dto.chain.as_deref().unwrap_or("All chains");
    let mut out = format!(
        "Historical TVL: {} ({} points)\n\n",
        label,
        dto.points.len()
    );
    if dto.points.is_empty() {
        out.push_str("  No data.");
        return out;
    }
    // Show last 20 points in pretty mode
    let start = dto.points.len().saturating_sub(20);
    out.push_str(&format!("{:<12} {:>14}\n", "DATE", "TVL"));
    out.push_str(&"-".repeat(28));
    out.push('\n');
    for p in &dto.points[start..] {
        let date = format_unix_date(p.date);
        out.push_str(&format!("{:<12} {:>14}\n", date, fmt_usd(Some(p.tvl_usd))));
    }
    if start > 0 {
        out.push_str(&format!(
            "\n  ({start} earlier points omitted — use --json for full data)"
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_coins_price(dto: &dto::CoinsPriceDto) -> String {
    if dto.prices.is_empty() {
        return "No prices found.".to_owned();
    }
    let mut out = format!(
        "{:<40} {:<8} {:>14} {:>6}\n",
        "COIN", "SYMBOL", "PRICE", "CONF"
    );
    out.push_str(&"-".repeat(72));
    out.push('\n');
    for p in &dto.prices {
        let price = p
            .price_usd
            .map(|v| format!("${:.6}", v))
            .unwrap_or_else(|| "-".to_owned());
        let conf = p
            .confidence
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "-".to_owned());
        out.push_str(&format!(
            "{:<40} {:<8} {:>14} {:>6}\n",
            truncate(&p.coin, 39),
            p.symbol.as_deref().unwrap_or("-"),
            price,
            conf,
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_coins_chart(dto: &dto::CoinsChartDto) -> String {
    if dto.coins.is_empty() {
        return "No chart data found.".to_owned();
    }
    let mut out = String::new();
    for entry in &dto.coins {
        out.push_str(&format!(
            "{} ({}) — {} points\n",
            entry.coin,
            entry.symbol.as_deref().unwrap_or("?"),
            entry.points.len(),
        ));
        // Show last 10 points
        let start = entry.points.len().saturating_sub(10);
        out.push_str(&format!("  {:<12} {:>14}\n", "DATE", "PRICE"));
        out.push_str(&format!("  {}\n", "-".repeat(28)));
        for p in &entry.points[start..] {
            let date = format_unix_date(p.timestamp);
            out.push_str(&format!(
                "  {:<12} {:>14}\n",
                date,
                format!("${:.6}", p.price_usd)
            ));
        }
        if start > 0 {
            out.push_str(&format!("  ({start} earlier points omitted)\n"));
        }
        out.push('\n');
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_coins_change(dto: &dto::CoinsChangeDto) -> String {
    let period = dto.period.as_deref().unwrap_or("?");
    let mut out = format!("Price change ({period})\n\n");
    out.push_str(&format!("{:<40} {:>10}\n", "COIN", "CHANGE"));
    out.push_str(&"-".repeat(52));
    out.push('\n');
    for c in &dto.coins {
        out.push_str(&format!(
            "{:<40} {:>10}\n",
            truncate(&c.coin, 39),
            fmt_pct(Some(c.change_pct))
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_block(dto: &dto::BlockDto) -> String {
    let height = dto
        .height
        .map(|h| h.to_string())
        .unwrap_or_else(|| "-".to_owned());
    let ts = dto
        .timestamp
        .map(format_unix_date)
        .unwrap_or_else(|| "-".to_owned());
    format!("{}: block {} ({})", dto.chain, height, ts)
}

fn pretty_coins_liquidity(dto: &dto::CoinLiquidityDto) -> String {
    let mut out = format!("Liquidity: {} ({} points)\n\n", dto.token, dto.points.len());
    if dto.points.is_empty() {
        out.push_str("  No data.");
        return out;
    }
    let start = dto.points.len().saturating_sub(20);
    out.push_str(&format!("{:<12} {:>14}\n", "DATE", "LIQUIDITY"));
    out.push_str(&"-".repeat(28));
    out.push('\n');
    for p in &dto.points[start..] {
        out.push_str(&format!(
            "{:<12} {:>14}\n",
            format_unix_date(p.date),
            fmt_usd(Some(p.liquidity_usd))
        ));
    }
    if start > 0 {
        out.push_str(&format!(
            "\n  ({start} earlier points omitted — use --json for full data)"
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_stablecoins_list(dto: &dto::StablecoinsListDto) -> String {
    let mut out = format!(
        "{:<12} {:<8} {:>14} {:<12} {}\n",
        "NAME", "SYMBOL", "CIRCULATING", "PEG TYPE", "CHAINS"
    );
    out.push_str(&"-".repeat(70));
    out.push('\n');
    for s in &dto.stablecoins {
        let chains_str = if s.chains.len() <= 3 {
            s.chains.join(", ")
        } else {
            format!("{} (+{})", s.chains[..3].join(", "), s.chains.len() - 3)
        };
        out.push_str(&format!(
            "{:<12} {:<8} {:>14} {:<12} {}\n",
            truncate(&s.name, 11),
            truncate(&s.symbol, 7),
            fmt_usd(s.circulating_usd),
            s.peg_type.as_deref().unwrap_or("-"),
            truncate(&chains_str, 30),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_stablecoin_detail(dto: &dto::StablecoinDetailDto) -> String {
    let mut out = format!("{} ({})\n", dto.name, dto.symbol);
    if let Some(peg) = &dto.peg_type {
        out.push_str(&format!("Peg type:      {peg}\n"));
    }
    if let Some(mech) = &dto.peg_mechanism {
        out.push_str(&format!("Peg mechanism: {mech}\n"));
    }
    if let Some(price) = dto.price {
        out.push_str(&format!("Price:         ${:.4}\n", price));
    }
    if !dto.chains.is_empty() {
        out.push_str(&format!("Chains:        {}\n", dto.chains.join(", ")));
    }
    if !dto.chain_circulating.is_empty() {
        out.push_str("Chain circulating:\n");
        let mut sorted: Vec<_> = dto.chain_circulating.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (chain, circ) in sorted {
            out.push_str(&format!("  {:<20} {}\n", chain, fmt_usd(Some(*circ))));
        }
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_stablecoin_chains(dto: &dto::StablecoinChainsDto) -> String {
    let mut out = format!("{:<25} {:>14}\n", "CHAIN", "CIRCULATING");
    out.push_str(&"-".repeat(42));
    out.push('\n');
    for c in &dto.chains {
        out.push_str(&format!(
            "{:<25} {:>14}\n",
            truncate(&c.name, 24),
            fmt_usd(c.circulating_usd),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_stablecoin_chart(dto: &dto::StablecoinChartDto) -> String {
    let label = dto.chain.as_deref().unwrap_or("All chains");
    let mut out = format!(
        "Stablecoin mcap: {} ({} points)\n\n",
        label,
        dto.points.len()
    );
    if dto.points.is_empty() {
        out.push_str("  No data.");
        return out;
    }
    let start = dto.points.len().saturating_sub(20);
    out.push_str(&format!("{:<12} {:>14}\n", "DATE", "CIRCULATING"));
    out.push_str(&"-".repeat(28));
    out.push('\n');
    for p in &dto.points[start..] {
        out.push_str(&format!(
            "{:<12} {:>14}\n",
            format_unix_date(p.date),
            fmt_usd(Some(p.circulating_usd)),
        ));
    }
    if start > 0 {
        out.push_str(&format!(
            "\n  ({start} earlier points omitted — use --json for full data)"
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_stablecoin_dominance(dto: &dto::StablecoinDominanceDto) -> String {
    let mut out = format!(
        "Stablecoin dominance: {} ({} points)\n\n",
        dto.chain,
        dto.points.len()
    );
    if dto.points.is_empty() {
        out.push_str("  No data.");
        return out;
    }
    if let Some(latest) = dto.points.last() {
        out.push_str(&format!("Latest ({}):\n", format_unix_date(latest.date)));
        if let Some(total) = latest.total_circulating_usd {
            out.push_str(&format!("  Total: {}\n", fmt_usd(Some(total))));
        }
        let mut sorted = latest.dominance.clone();
        sorted.sort_by(|a, b| {
            b.dominance_pct
                .partial_cmp(&a.dominance_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out.push_str(&format!("\n  {:<12} {:>10}\n", "STABLECOIN", "SHARE"));
        out.push_str(&format!("  {}\n", "-".repeat(24)));
        for e in &sorted {
            out.push_str(&format!(
                "  {:<12} {:>9.2}%\n",
                truncate(&e.name, 11),
                e.dominance_pct,
            ));
        }
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_stablecoin_prices(dto: &dto::StablecoinPricesDto) -> String {
    if dto.points.is_empty() {
        return "No stablecoin price data.".to_owned();
    }
    let latest = &dto.points[dto.points.len() - 1];
    let mut out = format!("Stablecoin prices ({})\n\n", format_unix_date(latest.date));
    out.push_str(&format!("{:<12} {:>12}\n", "STABLECOIN", "PRICE"));
    out.push_str(&"-".repeat(26));
    out.push('\n');
    for p in &latest.prices {
        out.push_str(&format!(
            "{:<12} {:>12}\n",
            truncate(&p.name, 11),
            format!("${:.4}", p.price),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_bridges_list(dto: &dto::BridgesListDto) -> String {
    let mut out = format!(
        "{:<4} {:<25} {:>14} {:>14} {:>14}\n",
        "ID", "NAME", "DAILY VOL", "WEEKLY VOL", "MONTHLY VOL"
    );
    out.push_str(&"-".repeat(75));
    out.push('\n');
    for b in &dto.bridges {
        out.push_str(&format!(
            "{:<4} {:<25} {:>14} {:>14} {:>14}\n",
            b.id,
            truncate(&b.name, 24),
            fmt_usd(b.daily_volume_usd),
            fmt_usd(b.weekly_volume_usd),
            fmt_usd(b.monthly_volume_usd),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_bridge_detail(dto: &dto::BridgeDetailDto) -> String {
    let mut out = format!("{} (id: {})\n", dto.name, dto.id);
    if !dto.chains.is_empty() {
        out.push_str(&format!("Chains:      {}\n", dto.chains.join(", ")));
    }
    if let Some(dest) = &dto.destination_chain {
        out.push_str(&format!("Destination: {dest}\n"));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_bridge_volume(dto: &dto::BridgeVolumeDto) -> String {
    let mut out = format!(
        "Bridge volume: {} ({} points)\n\n",
        dto.chain,
        dto.points.len()
    );
    if dto.points.is_empty() {
        out.push_str("  No data.");
        return out;
    }
    let start = dto.points.len().saturating_sub(20);
    out.push_str(&format!(
        "{:<12} {:>14} {:>14} {:>6} {:>6}\n",
        "DATE", "DEPOSITS", "WITHDRAWALS", "D_TX", "W_TX"
    ));
    out.push_str(&"-".repeat(56));
    out.push('\n');
    for p in &dto.points[start..] {
        out.push_str(&format!(
            "{:<12} {:>14} {:>14} {:>6} {:>6}\n",
            format_unix_date(p.date),
            fmt_usd(p.deposit_usd),
            fmt_usd(p.withdraw_usd),
            p.deposit_txs
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            p.withdraw_txs
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".to_owned()),
        ));
    }
    if start > 0 {
        out.push_str(&format!(
            "\n  ({start} earlier points omitted — use --json for full data)"
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_bridge_daystats(dto: &dto::BridgeDayStatsDto) -> String {
    let mut out = format!(
        "Bridge day stats: {} ({})\n\n",
        dto.chain,
        format_unix_date(dto.timestamp)
    );
    if dto.stats.is_empty() {
        out.push_str("  No data.");
        return out;
    }
    out.push_str(&format!(
        "{:<12} {:>10} {:>10} {:>10} {:>10}\n",
        "DATE", "TOK_DEP", "TOK_WD", "ADDR_DEP", "ADDR_WD"
    ));
    out.push_str(&"-".repeat(56));
    out.push('\n');
    for s in &dto.stats {
        out.push_str(&format!(
            "{:<12} {:>10} {:>10} {:>10} {:>10}\n",
            format_unix_date(s.date),
            s.tokens_deposited_count,
            s.tokens_withdrawn_count,
            s.addresses_deposited_count,
            s.addresses_withdrawn_count,
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_bridge_tx(dto: &dto::BridgeTxDto) -> String {
    let mut out = format!(
        "Bridge {} transactions ({} shown)\n\n",
        dto.bridge_id,
        dto.transactions.len()
    );
    if dto.transactions.is_empty() {
        out.push_str("  No transactions.");
        return out;
    }
    out.push_str(&format!(
        "{:<14} {:<8} {:<8} {:>12} {:<6} {}\n",
        "DATE", "CHAIN", "TOKEN", "AMOUNT", "TYPE", "TX HASH"
    ));
    out.push_str(&"-".repeat(72));
    out.push('\n');
    for tx in &dto.transactions {
        let date = tx
            .timestamp
            .map(format_unix_date)
            .unwrap_or_else(|| "-".to_owned());
        let tx_type = match tx.is_deposit {
            Some(true) => "IN",
            Some(false) => "OUT",
            None => "?",
        };
        out.push_str(&format!(
            "{:<14} {:<8} {:<8} {:>12} {:<6} {}\n",
            date,
            tx.chain.as_deref().unwrap_or("-"),
            tx.token.as_deref().unwrap_or("-"),
            tx.amount.as_deref().unwrap_or("-"),
            tx_type,
            truncate(tx.tx_hash.as_str(), 20),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_overview(dto: &dto::OverviewDto) -> String {
    let label = match dto.chain.as_deref() {
        Some(c) => format!("{} ({})", dto.metric, c),
        None => dto.metric.clone(),
    };
    let mut out = format!(
        "{label}\n  24h total: {}  7d total: {}\n  1d change: {}  7d change: {}\n\n",
        fmt_usd(dto.total_24h_usd),
        fmt_usd(dto.total_7d_usd),
        fmt_pct(dto.change_1d_pct),
        fmt_pct(dto.change_7d_pct),
    );
    if dto.protocols.is_empty() {
        out.push_str("  No protocol data.");
        return out;
    }
    out.push_str(&format!(
        "{:<25} {:<10} {:>12} {:>8} {:>8}\n",
        "PROTOCOL", "CATEGORY", "24H", "1D", "7D"
    ));
    out.push_str(&"-".repeat(67));
    out.push('\n');
    for p in &dto.protocols {
        out.push_str(&format!(
            "{:<25} {:<10} {:>12} {:>8} {:>8}\n",
            truncate(&p.name, 24),
            truncate(p.category.as_deref().unwrap_or("-"), 9),
            fmt_usd(p.total_24h_usd),
            fmt_pct(p.change_1d_pct),
            fmt_pct(p.change_7d_pct),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_protocol_summary(dto: &dto::ProtocolSummaryDto) -> String {
    let mut out = format!("{} — {} summary\n", dto.name, dto.metric);
    if let Some(cat) = &dto.category {
        out.push_str(&format!("Category:  {cat}\n"));
    }
    out.push_str(&format!("24h:       {}\n", fmt_usd(dto.total_24h_usd)));
    out.push_str(&format!("7d:        {}\n", fmt_usd(dto.total_7d_usd)));
    out.push_str(&format!("30d:       {}\n", fmt_usd(dto.total_30d_usd)));
    out.push_str(&format!("1d change: {}\n", fmt_pct(dto.change_1d_pct)));
    out.push_str(&format!("7d change: {}\n", fmt_pct(dto.change_7d_pct)));
    if !dto.chains.is_empty() {
        out.push_str(&format!("Chains:    {}\n", dto.chains.join(", ")));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_yield_pools(dto: &dto::YieldPoolsDto) -> String {
    let mut out = format!(
        "{:<18} {:<12} {:<10} {:>8} {:>8} {:>12}\n",
        "PROJECT", "CHAIN", "SYMBOL", "APY", "BASE", "TVL"
    );
    out.push_str(&"-".repeat(72));
    out.push('\n');
    for p in &dto.pools {
        out.push_str(&format!(
            "{:<18} {:<12} {:<10} {:>8} {:>8} {:>12}\n",
            truncate(p.project.as_deref().unwrap_or("-"), 17),
            truncate(p.chain.as_deref().unwrap_or("-"), 11),
            truncate(p.symbol.as_deref().unwrap_or("-"), 9),
            p.apy
                .map(|v| format!("{:.2}%", v))
                .unwrap_or_else(|| "-".to_owned()),
            p.apy_base
                .map(|v| format!("{:.2}%", v))
                .unwrap_or_else(|| "-".to_owned()),
            fmt_usd(p.tvl_usd),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_yield_borrow_pools(dto: &dto::YieldBorrowPoolsDto) -> String {
    let mut out = format!(
        "{:<18} {:<12} {:<8} {:>8} {:>8} {:>12} {:>12}\n",
        "PROJECT", "CHAIN", "SYMBOL", "APY", "BORROW", "SUPPLY", "BORROW$"
    );
    out.push_str(&"-".repeat(82));
    out.push('\n');
    for p in &dto.pools {
        let borrow_apy = match (p.apy_base_borrow, p.apy_reward_borrow) {
            (Some(base), Some(reward)) => format!("{:.2}%", base + reward),
            (Some(base), None) => format!("{:.2}%", base),
            _ => "-".to_owned(),
        };
        out.push_str(&format!(
            "{:<18} {:<12} {:<8} {:>8} {:>8} {:>12} {:>12}\n",
            truncate(p.project.as_deref().unwrap_or("-"), 17),
            truncate(p.chain.as_deref().unwrap_or("-"), 11),
            truncate(p.symbol.as_deref().unwrap_or("-"), 7),
            p.apy
                .map(|v| format!("{:.2}%", v))
                .unwrap_or_else(|| "-".to_owned()),
            borrow_apy,
            fmt_usd(p.total_supply_usd),
            fmt_usd(p.total_borrow_usd),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_yield_chart(dto: &dto::YieldChartDto) -> String {
    let mut out = format!(
        "Yield chart: {} ({} points)\n\n",
        dto.pool,
        dto.points.len()
    );
    if dto.points.is_empty() {
        out.push_str("  No data.");
        return out;
    }
    let start = dto.points.len().saturating_sub(20);
    out.push_str(&format!(
        "{:<12} {:>8} {:>8} {:>12}\n",
        "DATE", "APY", "BASE", "TVL"
    ));
    out.push_str(&"-".repeat(44));
    out.push('\n');
    for p in &dto.points[start..] {
        let date = if p.timestamp.len() >= 10 {
            &p.timestamp[..10]
        } else {
            &p.timestamp
        };
        out.push_str(&format!(
            "{:<12} {:>8} {:>8} {:>12}\n",
            date,
            p.apy
                .map(|v| format!("{:.2}%", v))
                .unwrap_or_else(|| "-".to_owned()),
            p.apy_base
                .map(|v| format!("{:.2}%", v))
                .unwrap_or_else(|| "-".to_owned()),
            fmt_usd(p.tvl_usd),
        ));
    }
    if start > 0 {
        out.push_str(&format!(
            "\n  ({start} earlier points omitted — use --json for full data)"
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_yield_lend_borrow_chart(dto: &dto::YieldLendBorrowChartDto) -> String {
    let mut out = format!(
        "Lend/borrow chart: {} ({} points)\n\n",
        dto.pool,
        dto.points.len()
    );
    if dto.points.is_empty() {
        out.push_str("  No data.");
        return out;
    }
    let start = dto.points.len().saturating_sub(20);
    out.push_str(&format!(
        "{:<12} {:>8} {:>8} {:>12} {:>12}\n",
        "DATE", "APY", "BORROW", "SUPPLY$", "BORROW$"
    ));
    out.push_str(&"-".repeat(56));
    out.push('\n');
    for p in &dto.points[start..] {
        let date = if p.timestamp.len() >= 10 {
            &p.timestamp[..10]
        } else {
            &p.timestamp
        };
        let borrow = match (p.apy_base_borrow, p.apy_reward_borrow) {
            (Some(b), Some(r)) => format!("{:.2}%", b + r),
            (Some(b), None) => format!("{:.2}%", b),
            _ => "-".to_owned(),
        };
        out.push_str(&format!(
            "{:<12} {:>8} {:>8} {:>12} {:>12}\n",
            date,
            p.apy
                .map(|v| format!("{:.2}%", v))
                .unwrap_or_else(|| "-".to_owned()),
            borrow,
            fmt_usd(p.total_supply_usd),
            fmt_usd(p.total_borrow_usd),
        ));
    }
    if start > 0 {
        out.push_str(&format!(
            "\n  ({start} earlier points omitted — use --json for full data)"
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_perps(dto: &dto::PerpsDto) -> String {
    let mut out = format!(
        "{:<18} {:<10} {:>12} {:>14} {:>12}\n",
        "MARKETPLACE", "SYMBOL", "FUNDING", "OPEN INT", "PRICE"
    );
    out.push_str(&"-".repeat(70));
    out.push('\n');
    for p in &dto.perps {
        out.push_str(&format!(
            "{:<18} {:<10} {:>12} {:>14} {:>12}\n",
            truncate(p.marketplace.as_deref().unwrap_or("-"), 17),
            truncate(p.symbol.as_deref().unwrap_or("-"), 9),
            p.funding_rate
                .map(|v| format!("{:.6}", v))
                .unwrap_or_else(|| "-".to_owned()),
            fmt_usd(p.open_interest),
            p.index_price
                .map(|v| format!("${:.2}", v))
                .unwrap_or_else(|| "-".to_owned()),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_lsd(dto: &dto::LsdDto) -> String {
    let mut out = format!(
        "{:<20} {:<8} {:>8} {:>10} {:>8}\n",
        "NAME", "SYMBOL", "APY", "SHARE", "ETH PEG"
    );
    out.push_str(&"-".repeat(58));
    out.push('\n');
    for r in &dto.rates {
        out.push_str(&format!(
            "{:<20} {:<8} {:>8} {:>10} {:>8}\n",
            truncate(&r.name, 19),
            truncate(r.symbol.as_deref().unwrap_or("-"), 7),
            r.apy
                .map(|v| format!("{:.2}%", v))
                .unwrap_or_else(|| "-".to_owned()),
            r.market_share
                .map(|v| format!("{:.2}%", v))
                .unwrap_or_else(|| "-".to_owned()),
            r.eth_peg
                .map(|v| format!("{:.4}", v))
                .unwrap_or_else(|| "-".to_owned()),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

// ---------------------------------------------------------------------------
// Ecosystem pretty formatters
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_unix_date(ts: i64) -> String {
    // Simple date formatting without chrono dependency.
    // Convert unix timestamp to YYYY-MM-DD.
    const SECS_PER_DAY: i64 = 86400;
    let days = ts / SECS_PER_DAY;

    // Civil date from day count (algorithm from Howard Hinnant)
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}

// ---------------------------------------------------------------------------
// Usage command implementation
// ---------------------------------------------------------------------------

async fn cmd_usage(ctx: &Ctx) -> Result<(), HeatError> {
    let api_key = config::require_api_key(&ctx.config)?;
    let client = make_client(ctx)?;
    let raw = client.usage(&api_key).await?;
    let dto = map::map_usage(raw);
    ctx.output
        .write_data(&dto, Some(&pretty_usage))
        .map_err(io_err)
}

fn pretty_usage(dto: &dto::UsageDto) -> String {
    let mut out = "API Usage\n".to_owned();
    out.push_str(&format!(
        "Requests today:       {}\n",
        dto.requests_today
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_owned())
    ));
    out.push_str(&format!(
        "Requests this month:  {}\n",
        dto.requests_this_month
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_owned())
    ));
    out.push_str(&format!(
        "Rate limit:           {}\n",
        dto.rate_limit
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_owned())
    ));
    out.trim_end_matches('\n').to_owned()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_usd_billions() {
        assert_eq!(fmt_usd(Some(5_200_000_000.0)), "$5.20B");
    }

    #[test]
    fn fmt_usd_millions() {
        assert_eq!(fmt_usd(Some(42_500_000.0)), "$42.50M");
    }

    #[test]
    fn fmt_usd_thousands() {
        assert_eq!(fmt_usd(Some(1_234.56)), "$1.23K");
    }

    #[test]
    fn fmt_usd_small() {
        assert_eq!(fmt_usd(Some(42.5)), "$42.50");
    }

    #[test]
    fn fmt_usd_none() {
        assert_eq!(fmt_usd(None), "-");
    }

    #[test]
    fn fmt_pct_positive() {
        assert_eq!(fmt_pct(Some(5.23)), "+5.23%");
    }

    #[test]
    fn fmt_pct_negative() {
        assert_eq!(fmt_pct(Some(-3.1)), "-3.10%");
    }

    #[test]
    fn format_unix_date_epoch() {
        assert_eq!(format_unix_date(0), "1970-01-01");
    }

    #[test]
    fn format_unix_date_known() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(format_unix_date(1704067200), "2024-01-01");
    }

    #[test]
    fn sort_protocols_by_tvl() {
        let mut protocols = vec![
            dto::ProtocolRow {
                id: "1".into(),
                slug: "a".into(),
                name: "A".into(),
                symbol: None,
                category: None,
                chains: vec![],
                tvl_usd: Some(100.0),
                change_1d_pct: None,
                change_7d_pct: None,
                change_1m_pct: None,
                url: None,
            },
            dto::ProtocolRow {
                id: "2".into(),
                slug: "b".into(),
                name: "B".into(),
                symbol: None,
                category: None,
                chains: vec![],
                tvl_usd: Some(200.0),
                change_1d_pct: None,
                change_7d_pct: None,
                change_1m_pct: None,
                url: None,
            },
        ];
        sort_protocols(&mut protocols, "tvl");
        assert_eq!(protocols[0].slug, "b"); // higher TVL first
    }

    #[test]
    fn sort_protocols_by_name() {
        let mut protocols = vec![
            dto::ProtocolRow {
                id: "1".into(),
                slug: "b".into(),
                name: "Bbb".into(),
                symbol: None,
                category: None,
                chains: vec![],
                tvl_usd: None,
                change_1d_pct: None,
                change_7d_pct: None,
                change_1m_pct: None,
                url: None,
            },
            dto::ProtocolRow {
                id: "2".into(),
                slug: "a".into(),
                name: "Aaa".into(),
                symbol: None,
                category: None,
                chains: vec![],
                tvl_usd: None,
                change_1d_pct: None,
                change_7d_pct: None,
                change_1m_pct: None,
                url: None,
            },
        ];
        sort_protocols(&mut protocols, "name");
        assert_eq!(protocols[0].slug, "a");
    }

    #[test]
    fn validate_sort_accepts_valid() {
        assert!(validate_sort("tvl", PROTOCOL_SORT_VALUES).is_ok());
        assert!(validate_sort("name", PROTOCOL_SORT_VALUES).is_ok());
        assert!(validate_sort("change_1d", PROTOCOL_SORT_VALUES).is_ok());
    }

    #[test]
    fn validate_sort_rejects_invalid() {
        let err = validate_sort("bogus", PROTOCOL_SORT_VALUES).unwrap_err();
        assert_eq!(err.reason, "invalid_sort");
    }
}
