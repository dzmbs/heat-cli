//! Data commands — on-chain analytics, positions, trades, leaderboards.

use clap::Subcommand;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use polymarket_client_sdk::data;
use polymarket_client_sdk::data::types::request::{
    ActivityRequest, BuilderLeaderboardRequest, BuilderVolumeRequest, ClosedPositionsRequest,
    HoldersRequest, LiveVolumeRequest, OpenInterestRequest, PositionsRequest, TradedRequest,
    TraderLeaderboardRequest, TradesRequest, ValueRequest,
};
use polymarket_client_sdk::data::types::response::{
    Activity, BuilderLeaderboardEntry, BuilderVolumeEntry, ClosedPosition, Holder, LiveVolume,
    Market, MarketVolume, MetaHolder, OpenInterest, Position, Trade, Traded,
    TraderLeaderboardEntry, Value,
};
use polymarket_client_sdk::data::types::{ActivityType, LeaderboardOrderBy, Side, TimePeriod};
use polymarket_client_sdk::types::{Address, B256};
use serde::Serialize;

use crate::auth;

// ── helpers ──────────────────────────────────────────────────────────────────

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}

fn data_err(e: impl std::fmt::Display) -> HeatError {
    HeatError::network("data_request", format!("Data API error: {e}"))
}

fn bound_err(e: impl std::fmt::Display) -> HeatError {
    HeatError::validation("invalid_param", format!("Parameter out of range: {e}"))
}

fn parse_address(s: &str) -> Result<Address, HeatError> {
    s.parse::<Address>().map_err(|e| {
        HeatError::validation(
            "invalid_address",
            format!("Invalid Ethereum address '{s}': {e}"),
        )
        .with_hint("Address must be a hex string like 0x1234...abcd")
    })
}

fn parse_b256(s: &str) -> Result<B256, HeatError> {
    s.parse::<B256>().map_err(|e| {
        HeatError::validation(
            "invalid_condition_id",
            format!("Invalid condition ID '{s}': {e}"),
        )
        .with_hint("Condition ID must be a 32-byte hex string")
    })
}

fn parse_time_period(s: &str) -> Result<TimePeriod, HeatError> {
    match s.to_lowercase().as_str() {
        "day" | "1d" => Ok(TimePeriod::Day),
        "week" | "1w" => Ok(TimePeriod::Week),
        "month" | "1m" => Ok(TimePeriod::Month),
        "all" => Ok(TimePeriod::All),
        _ => Err(
            HeatError::validation("invalid_time_period", format!("Invalid time period: {s}"))
                .with_hint("Valid periods: day, week, month, all"),
        ),
    }
}

fn fmt_address(a: &Address) -> String {
    format!("{a:#x}")
}

fn fmt_b256(b: &B256) -> String {
    format!("{b:#x}")
}

fn fmt_u256(u: &polymarket_client_sdk::types::U256) -> String {
    u.to_string()
}

fn fmt_decimal(d: &polymarket_client_sdk::types::Decimal) -> String {
    d.to_string()
}

fn fmt_side(s: &Side) -> String {
    match s {
        Side::Buy => "buy".to_string(),
        Side::Sell => "sell".to_string(),
        Side::Unknown(raw) => raw.to_lowercase(),
        _ => "unknown".to_string(),
    }
}

fn fmt_activity_type(a: &ActivityType) -> String {
    match a {
        ActivityType::Trade => "trade".to_string(),
        ActivityType::Split => "split".to_string(),
        ActivityType::Merge => "merge".to_string(),
        ActivityType::Redeem => "redeem".to_string(),
        ActivityType::Reward => "reward".to_string(),
        ActivityType::Conversion => "conversion".to_string(),
        ActivityType::Yield => "yield".to_string(),
        ActivityType::MakerRebate => "maker_rebate".to_string(),
        ActivityType::Unknown(raw) => raw.to_lowercase(),
        _ => "unknown".to_string(),
    }
}

fn fmt_market(m: &Market) -> String {
    match m {
        Market::Global => "global".to_string(),
        Market::Market(cid) => fmt_b256(cid),
        _ => "unknown".to_string(),
    }
}

fn write_json(ctx: &Ctx, val: serde_json::Value) -> Result<(), HeatError> {
    ctx.output.write_data(&val, None).map_err(io_err)
}

// ── Heat-owned DTOs ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PositionDto {
    proxy_wallet: String,
    condition_id: String,
    asset: String,
    outcome: String,
    outcome_index: i32,
    size: String,
    avg_price: String,
    cur_price: String,
    current_value: String,
    initial_value: String,
    cash_pnl: String,
    percent_pnl: String,
    realized_pnl: String,
    total_bought: String,
    redeemable: bool,
    mergeable: bool,
    title: String,
    slug: String,
    event_slug: String,
    end_date: String,
    negative_risk: bool,
}

impl From<&Position> for PositionDto {
    fn from(p: &Position) -> Self {
        Self {
            proxy_wallet: fmt_address(&p.proxy_wallet),
            condition_id: fmt_b256(&p.condition_id),
            asset: fmt_u256(&p.asset),
            outcome: p.outcome.clone(),
            outcome_index: p.outcome_index,
            size: fmt_decimal(&p.size),
            avg_price: fmt_decimal(&p.avg_price),
            cur_price: fmt_decimal(&p.cur_price),
            current_value: fmt_decimal(&p.current_value),
            initial_value: fmt_decimal(&p.initial_value),
            cash_pnl: fmt_decimal(&p.cash_pnl),
            percent_pnl: fmt_decimal(&p.percent_pnl),
            realized_pnl: fmt_decimal(&p.realized_pnl),
            total_bought: fmt_decimal(&p.total_bought),
            redeemable: p.redeemable,
            mergeable: p.mergeable,
            title: p.title.clone(),
            slug: p.slug.clone(),
            event_slug: p.event_slug.clone(),
            end_date: p.end_date.to_string(),
            negative_risk: p.negative_risk,
        }
    }
}

#[derive(Serialize)]
struct ClosedPositionDto {
    proxy_wallet: String,
    condition_id: String,
    asset: String,
    outcome: String,
    outcome_index: i32,
    avg_price: String,
    cur_price: String,
    realized_pnl: String,
    total_bought: String,
    timestamp: i64,
    title: String,
    slug: String,
    event_slug: String,
    end_date: String,
}

impl From<&ClosedPosition> for ClosedPositionDto {
    fn from(p: &ClosedPosition) -> Self {
        Self {
            proxy_wallet: fmt_address(&p.proxy_wallet),
            condition_id: fmt_b256(&p.condition_id),
            asset: fmt_u256(&p.asset),
            outcome: p.outcome.clone(),
            outcome_index: p.outcome_index,
            avg_price: fmt_decimal(&p.avg_price),
            cur_price: fmt_decimal(&p.cur_price),
            realized_pnl: fmt_decimal(&p.realized_pnl),
            total_bought: fmt_decimal(&p.total_bought),
            timestamp: p.timestamp,
            title: p.title.clone(),
            slug: p.slug.clone(),
            event_slug: p.event_slug.clone(),
            end_date: p.end_date.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
struct TradeDto {
    proxy_wallet: String,
    side: String,
    condition_id: String,
    asset: String,
    outcome: String,
    outcome_index: i32,
    size: String,
    price: String,
    timestamp: i64,
    transaction_hash: String,
    title: String,
    slug: String,
    event_slug: String,
    name: Option<String>,
    pseudonym: Option<String>,
}

impl From<&Trade> for TradeDto {
    fn from(t: &Trade) -> Self {
        Self {
            proxy_wallet: fmt_address(&t.proxy_wallet),
            side: fmt_side(&t.side),
            condition_id: fmt_b256(&t.condition_id),
            asset: fmt_u256(&t.asset),
            outcome: t.outcome.clone(),
            outcome_index: t.outcome_index,
            size: fmt_decimal(&t.size),
            price: fmt_decimal(&t.price),
            timestamp: t.timestamp,
            transaction_hash: fmt_b256(&t.transaction_hash),
            title: t.title.clone(),
            slug: t.slug.clone(),
            event_slug: t.event_slug.clone(),
            name: t.name.clone(),
            pseudonym: t.pseudonym.clone(),
        }
    }
}

#[derive(Serialize)]
struct ActivityDto {
    proxy_wallet: String,
    timestamp: i64,
    activity_type: String,
    size: String,
    usdc_size: String,
    transaction_hash: String,
    condition_id: Option<String>,
    asset: Option<String>,
    side: Option<String>,
    price: Option<String>,
    outcome_index: Option<i32>,
    title: Option<String>,
    slug: Option<String>,
    event_slug: Option<String>,
    outcome: Option<String>,
    name: Option<String>,
    pseudonym: Option<String>,
}

impl From<&Activity> for ActivityDto {
    fn from(a: &Activity) -> Self {
        Self {
            proxy_wallet: fmt_address(&a.proxy_wallet),
            timestamp: a.timestamp,
            activity_type: fmt_activity_type(&a.activity_type),
            size: fmt_decimal(&a.size),
            usdc_size: fmt_decimal(&a.usdc_size),
            transaction_hash: fmt_b256(&a.transaction_hash),
            condition_id: a.condition_id.as_ref().map(fmt_b256),
            asset: a.asset.as_ref().map(fmt_u256),
            side: a.side.as_ref().map(fmt_side),
            price: a.price.as_ref().map(fmt_decimal),
            outcome_index: a.outcome_index,
            title: a.title.clone(),
            slug: a.slug.clone(),
            event_slug: a.event_slug.clone(),
            outcome: a.outcome.clone(),
            name: a.name.clone(),
            pseudonym: a.pseudonym.clone(),
        }
    }
}

#[derive(Serialize)]
struct HolderDto {
    proxy_wallet: String,
    asset: String,
    outcome_index: i32,
    amount: String,
    name: Option<String>,
    pseudonym: Option<String>,
    verified: Option<bool>,
    display_username_public: Option<bool>,
}

impl From<&Holder> for HolderDto {
    fn from(h: &Holder) -> Self {
        Self {
            proxy_wallet: fmt_address(&h.proxy_wallet),
            asset: fmt_u256(&h.asset),
            outcome_index: h.outcome_index,
            amount: fmt_decimal(&h.amount),
            name: h.name.clone(),
            pseudonym: h.pseudonym.clone(),
            verified: h.verified,
            display_username_public: h.display_username_public,
        }
    }
}

#[derive(Serialize)]
struct MetaHolderDto {
    token: String,
    holders: Vec<HolderDto>,
}

impl From<&MetaHolder> for MetaHolderDto {
    fn from(m: &MetaHolder) -> Self {
        Self {
            token: fmt_u256(&m.token),
            holders: m.holders.iter().map(HolderDto::from).collect(),
        }
    }
}

#[derive(Serialize)]
struct TradedDto {
    user: String,
    traded: i32,
}

impl From<&Traded> for TradedDto {
    fn from(t: &Traded) -> Self {
        Self {
            user: fmt_address(&t.user),
            traded: t.traded,
        }
    }
}

#[derive(Serialize)]
struct ValueDto {
    user: String,
    value: String,
}

impl From<&Value> for ValueDto {
    fn from(v: &Value) -> Self {
        Self {
            user: fmt_address(&v.user),
            value: fmt_decimal(&v.value),
        }
    }
}

#[derive(Serialize)]
struct OpenInterestDto {
    market: String,
    value: String,
}

impl From<&OpenInterest> for OpenInterestDto {
    fn from(oi: &OpenInterest) -> Self {
        Self {
            market: fmt_market(&oi.market),
            value: fmt_decimal(&oi.value),
        }
    }
}

#[derive(Serialize)]
struct MarketVolumeDto {
    market: String,
    value: String,
}

#[derive(Serialize)]
struct LiveVolumeDto {
    total: String,
    markets: Vec<MarketVolumeDto>,
}

impl From<&LiveVolume> for LiveVolumeDto {
    fn from(v: &LiveVolume) -> Self {
        Self {
            total: fmt_decimal(&v.total),
            markets: v
                .markets
                .iter()
                .map(|mv| MarketVolumeDto::from(mv))
                .collect(),
        }
    }
}

impl From<&MarketVolume> for MarketVolumeDto {
    fn from(mv: &MarketVolume) -> Self {
        Self {
            market: fmt_market(&mv.market),
            value: fmt_decimal(&mv.value),
        }
    }
}

#[derive(Serialize)]
struct TraderLeaderboardEntryDto {
    rank: i32,
    proxy_wallet: String,
    user_name: Option<String>,
    vol: String,
    pnl: String,
    x_username: Option<String>,
    verified_badge: Option<bool>,
}

impl From<&TraderLeaderboardEntry> for TraderLeaderboardEntryDto {
    fn from(e: &TraderLeaderboardEntry) -> Self {
        Self {
            rank: e.rank,
            proxy_wallet: fmt_address(&e.proxy_wallet),
            user_name: e.user_name.clone(),
            vol: fmt_decimal(&e.vol),
            pnl: fmt_decimal(&e.pnl),
            x_username: e.x_username.clone(),
            verified_badge: e.verified_badge,
        }
    }
}

#[derive(Serialize)]
struct BuilderLeaderboardEntryDto {
    rank: i32,
    builder: String,
    volume: String,
    active_users: i32,
    verified: bool,
}

impl From<&BuilderLeaderboardEntry> for BuilderLeaderboardEntryDto {
    fn from(e: &BuilderLeaderboardEntry) -> Self {
        Self {
            rank: e.rank,
            builder: e.builder.clone(),
            volume: fmt_decimal(&e.volume),
            active_users: e.active_users,
            verified: e.verified,
        }
    }
}

#[derive(Serialize)]
struct BuilderVolumeEntryDto {
    dt: String,
    builder: String,
    verified: bool,
    volume: String,
    active_users: i32,
    rank: i32,
}

impl From<&BuilderVolumeEntry> for BuilderVolumeEntryDto {
    fn from(e: &BuilderVolumeEntry) -> Self {
        Self {
            dt: e.dt.to_rfc3339(),
            builder: e.builder.clone(),
            verified: e.verified,
            volume: fmt_decimal(&e.volume),
            active_users: e.active_users,
            rank: e.rank,
        }
    }
}

// ── CLI subcommands ───────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum DataSubcommand {
    /// Get open positions for a user
    Positions {
        #[arg(long)]
        user: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: i32,
        #[arg(long)]
        offset: Option<i32>,
    },
    /// Get closed positions for a user
    ClosedPositions {
        #[arg(long)]
        user: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: i32,
        #[arg(long)]
        offset: Option<i32>,
    },
    /// Get portfolio value
    Value {
        #[arg(long)]
        user: Option<String>,
    },
    /// Get total amount traded
    Traded {
        #[arg(long)]
        user: Option<String>,
    },
    /// Get trade history
    Trades {
        #[arg(long)]
        user: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: i32,
        #[arg(long)]
        offset: Option<i32>,
    },
    /// Get activity feed
    Activity {
        #[arg(long)]
        user: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: i32,
        #[arg(long)]
        offset: Option<i32>,
    },
    /// Get holders for a market
    Holders {
        /// Condition ID(s), comma-separated
        markets: String,
        /// Max 20 per the API
        #[arg(long, default_value_t = 20)]
        limit: i32,
    },
    /// Get open interest for a market
    OpenInterest {
        /// Condition ID(s), comma-separated
        markets: String,
    },
    /// Get live volume for an event
    LiveVolume {
        /// Event ID (numeric)
        event_id: u64,
    },
    /// Trader leaderboard
    Leaderboard {
        #[arg(long, default_value = "all")]
        period: String,
        #[arg(long, default_value = "pnl")]
        order_by: String,
        /// Max 50 per the API
        #[arg(long, default_value_t = 25)]
        limit: i32,
        #[arg(long)]
        offset: Option<i32>,
    },
    /// Builder leaderboard
    BuilderLeaderboard {
        #[arg(long, default_value = "all")]
        period: String,
        /// Max 50 per the API
        #[arg(long, default_value_t = 25)]
        limit: i32,
        #[arg(long)]
        offset: Option<i32>,
    },
    /// Builder volume
    BuilderVolume {
        #[arg(long, default_value = "all")]
        period: String,
    },
}

// ── command dispatch ──────────────────────────────────────────────────────────

pub async fn run(sub: DataSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    let client = data::Client::default();

    match sub {
        DataSubcommand::Positions {
            user,
            limit,
            offset,
        } => {
            let addr = resolve_user(user, ctx)?;
            let req = if let Some(o) = offset {
                PositionsRequest::builder()
                    .user(addr)
                    .limit(limit)
                    .map_err(bound_err)?
                    .offset(o)
                    .map_err(bound_err)?
                    .build()
            } else {
                PositionsRequest::builder()
                    .user(addr)
                    .limit(limit)
                    .map_err(bound_err)?
                    .build()
            };
            let positions = client.positions(&req).await.map_err(data_err)?;
            let dtos: Vec<PositionDto> = positions.iter().map(PositionDto::from).collect();
            match ctx.output.format {
                OutputFormat::Pretty => {
                    if dtos.is_empty() {
                        println!("No open positions.");
                    } else {
                        for (i, (dto, raw)) in dtos.iter().zip(positions.iter()).enumerate() {
                            if i > 0 {
                                println!();
                            }
                            println!("  Market : {}", raw.title);
                            println!("  Outcome: {} (index {})", dto.outcome, dto.outcome_index);
                            println!("  Size   : {}", dto.size);
                            println!("  Price  : avg {} | cur {}", dto.avg_price, dto.cur_price);
                            println!(
                                "  Value  : {} (cost {})",
                                dto.current_value, dto.initial_value
                            );
                            println!("  PnL    : {} cash ({} %)", dto.cash_pnl, dto.percent_pnl);
                            if dto.redeemable {
                                println!("  [redeemable]");
                            }
                            if dto.mergeable {
                                println!("  [mergeable]");
                            }
                        }
                    }
                }
                _ => {
                    let val = serde_json::to_value(&dtos).map_err(|e| {
                        HeatError::internal("serialize", format!("Serialization failed: {e}"))
                    })?;
                    ctx.output.write_data(&val, None).map_err(io_err)?;
                }
            }
            Ok(())
        }

        DataSubcommand::ClosedPositions {
            user,
            limit,
            offset,
        } => {
            let addr = resolve_user(user, ctx)?;
            let req = if let Some(o) = offset {
                ClosedPositionsRequest::builder()
                    .user(addr)
                    .limit(limit)
                    .map_err(bound_err)?
                    .offset(o)
                    .map_err(bound_err)?
                    .build()
            } else {
                ClosedPositionsRequest::builder()
                    .user(addr)
                    .limit(limit)
                    .map_err(bound_err)?
                    .build()
            };
            let positions = client.closed_positions(&req).await.map_err(data_err)?;
            let dtos: Vec<ClosedPositionDto> =
                positions.iter().map(ClosedPositionDto::from).collect();
            match ctx.output.format {
                OutputFormat::Pretty => {
                    if dtos.is_empty() {
                        println!("No closed positions.");
                    } else {
                        for (dto, raw) in dtos.iter().zip(positions.iter()) {
                            println!("  Market    : {}", raw.title);
                            println!(
                                "  Outcome   : {} (index {})",
                                dto.outcome, dto.outcome_index
                            );
                            println!(
                                "  Avg price : {} | Close price: {}",
                                dto.avg_price, dto.cur_price
                            );
                            println!("  Realized  : {}", dto.realized_pnl);
                            println!("  Closed at : {}", dto.timestamp);
                            println!();
                        }
                    }
                }
                _ => {
                    let val = serde_json::to_value(&dtos).map_err(|e| {
                        HeatError::internal("serialize", format!("Serialization failed: {e}"))
                    })?;
                    ctx.output.write_data(&val, None).map_err(io_err)?;
                }
            }
            Ok(())
        }

        DataSubcommand::Value { user } => {
            let addr = resolve_user(user, ctx)?;
            let req = ValueRequest::builder().user(addr).build();
            let values = client.value(&req).await.map_err(data_err)?;
            let dtos: Vec<ValueDto> = values.iter().map(ValueDto::from).collect();
            match ctx.output.format {
                OutputFormat::Pretty => {
                    for dto in &dtos {
                        println!("  User  : {}", dto.user);
                        println!("  Value : {} USDC", dto.value);
                    }
                }
                _ => {
                    let val = serde_json::to_value(&dtos).map_err(|e| {
                        HeatError::internal("serialize", format!("Serialization failed: {e}"))
                    })?;
                    ctx.output.write_data(&val, None).map_err(io_err)?;
                }
            }
            Ok(())
        }

        DataSubcommand::Traded { user } => {
            let addr = resolve_user(user, ctx)?;
            let req = TradedRequest::builder().user(addr).build();
            let traded = client.traded(&req).await.map_err(data_err)?;
            let dto = TradedDto::from(&traded);
            let val = serde_json::to_value(&dto).map_err(|e| {
                HeatError::internal("serialize", format!("Serialization failed: {e}"))
            })?;
            write_json(ctx, val)
        }

        DataSubcommand::Trades {
            user,
            limit,
            offset,
        } => {
            let addr = resolve_user(user, ctx)?;
            let req = if let Some(o) = offset {
                TradesRequest::builder()
                    .user(addr)
                    .limit(limit)
                    .map_err(bound_err)?
                    .offset(o)
                    .map_err(bound_err)?
                    .build()
            } else {
                TradesRequest::builder()
                    .user(addr)
                    .limit(limit)
                    .map_err(bound_err)?
                    .build()
            };
            let trades = client.trades(&req).await.map_err(data_err)?;
            let dtos: Vec<TradeDto> = trades.iter().map(TradeDto::from).collect();
            match ctx.output.format {
                OutputFormat::Pretty => {
                    if dtos.is_empty() {
                        println!("No trades.");
                    } else {
                        for dto in &dtos {
                            println!(
                                "  {} {} @ {} x{} | {}",
                                dto.side, dto.outcome, dto.price, dto.size, dto.title
                            );
                        }
                    }
                }
                _ => {
                    let val = serde_json::to_value(&dtos).map_err(|e| {
                        HeatError::internal("serialize", format!("Serialization failed: {e}"))
                    })?;
                    ctx.output.write_data(&val, None).map_err(io_err)?;
                }
            }
            Ok(())
        }

        DataSubcommand::Activity {
            user,
            limit,
            offset,
        } => {
            let addr = resolve_user(user, ctx)?;
            let req = if let Some(o) = offset {
                ActivityRequest::builder()
                    .user(addr)
                    .limit(limit)
                    .map_err(bound_err)?
                    .offset(o)
                    .map_err(bound_err)?
                    .build()
            } else {
                ActivityRequest::builder()
                    .user(addr)
                    .limit(limit)
                    .map_err(bound_err)?
                    .build()
            };
            let activity = client.activity(&req).await.map_err(data_err)?;
            let dtos: Vec<ActivityDto> = activity.iter().map(ActivityDto::from).collect();
            match ctx.output.format {
                OutputFormat::Pretty => {
                    if dtos.is_empty() {
                        println!("No activity.");
                    } else {
                        for dto in &dtos {
                            let market = dto.title.as_deref().unwrap_or("-");
                            let outcome = dto.outcome.as_deref().unwrap_or("-");
                            println!(
                                "  [{}] {} {} {} USDC | {}",
                                dto.activity_type, market, outcome, dto.usdc_size, dto.timestamp
                            );
                        }
                    }
                }
                _ => {
                    let val = serde_json::to_value(&dtos).map_err(|e| {
                        HeatError::internal("serialize", format!("Serialization failed: {e}"))
                    })?;
                    ctx.output.write_data(&val, None).map_err(io_err)?;
                }
            }
            Ok(())
        }

        DataSubcommand::Holders { markets, limit } => {
            let market_list: Vec<B256> = markets
                .split(',')
                .map(|s| parse_b256(s.trim()))
                .collect::<Result<Vec<_>, _>>()?;
            // API max is 20; clamp to avoid BoundedIntError at runtime
            let limit = limit.min(20);
            let req = HoldersRequest::builder()
                .markets(market_list)
                .limit(limit)
                .map_err(bound_err)?
                .build();
            let holders = client.holders(&req).await.map_err(data_err)?;
            let dtos: Vec<MetaHolderDto> = holders.iter().map(MetaHolderDto::from).collect();
            let val = serde_json::to_value(&dtos).map_err(|e| {
                HeatError::internal("serialize", format!("Serialization failed: {e}"))
            })?;
            write_json(ctx, val)
        }

        DataSubcommand::OpenInterest { markets } => {
            let market_list: Vec<B256> = markets
                .split(',')
                .map(|s| parse_b256(s.trim()))
                .collect::<Result<Vec<_>, _>>()?;
            let req = OpenInterestRequest::builder().markets(market_list).build();
            let oi = client.open_interest(&req).await.map_err(data_err)?;
            let dtos: Vec<OpenInterestDto> = oi.iter().map(OpenInterestDto::from).collect();
            let val = serde_json::to_value(&dtos).map_err(|e| {
                HeatError::internal("serialize", format!("Serialization failed: {e}"))
            })?;
            write_json(ctx, val)
        }

        DataSubcommand::LiveVolume { event_id } => {
            let req = LiveVolumeRequest::builder().id(event_id).build();
            let vol = client.live_volume(&req).await.map_err(data_err)?;
            let dtos: Vec<LiveVolumeDto> = vol.iter().map(LiveVolumeDto::from).collect();
            let val = serde_json::to_value(&dtos).map_err(|e| {
                HeatError::internal("serialize", format!("Serialization failed: {e}"))
            })?;
            write_json(ctx, val)
        }

        DataSubcommand::Leaderboard {
            period,
            order_by,
            limit,
            offset,
        } => {
            let tp = parse_time_period(&period)?;
            let ob = match order_by.to_lowercase().as_str() {
                "pnl" => LeaderboardOrderBy::Pnl,
                "vol" => LeaderboardOrderBy::Vol,
                _ => {
                    return Err(HeatError::validation(
                        "invalid_order_by",
                        format!("Invalid order_by: {order_by}"),
                    )
                    .with_hint("Valid values: pnl, vol"));
                }
            };
            let req = if let Some(o) = offset {
                TraderLeaderboardRequest::builder()
                    .time_period(tp)
                    .order_by(ob)
                    .limit(limit)
                    .map_err(bound_err)?
                    .offset(o)
                    .map_err(bound_err)?
                    .build()
            } else {
                TraderLeaderboardRequest::builder()
                    .time_period(tp)
                    .order_by(ob)
                    .limit(limit)
                    .map_err(bound_err)?
                    .build()
            };
            let lb = client.leaderboard(&req).await.map_err(data_err)?;
            let dtos: Vec<TraderLeaderboardEntryDto> =
                lb.iter().map(TraderLeaderboardEntryDto::from).collect();
            match ctx.output.format {
                OutputFormat::Pretty => {
                    for dto in &dtos {
                        let name = dto.user_name.as_deref().unwrap_or(&dto.proxy_wallet);
                        println!(
                            "  #{:>3}  {:<30}  vol {}  pnl {}",
                            dto.rank, name, dto.vol, dto.pnl
                        );
                    }
                }
                _ => {
                    let val = serde_json::to_value(&dtos).map_err(|e| {
                        HeatError::internal("serialize", format!("Serialization failed: {e}"))
                    })?;
                    ctx.output.write_data(&val, None).map_err(io_err)?;
                }
            }
            Ok(())
        }

        DataSubcommand::BuilderLeaderboard {
            period,
            limit,
            offset,
        } => {
            let tp = parse_time_period(&period)?;
            let req = if let Some(o) = offset {
                BuilderLeaderboardRequest::builder()
                    .time_period(tp)
                    .limit(limit)
                    .map_err(bound_err)?
                    .offset(o)
                    .map_err(bound_err)?
                    .build()
            } else {
                BuilderLeaderboardRequest::builder()
                    .time_period(tp)
                    .limit(limit)
                    .map_err(bound_err)?
                    .build()
            };
            let lb = client.builder_leaderboard(&req).await.map_err(data_err)?;
            let dtos: Vec<BuilderLeaderboardEntryDto> =
                lb.iter().map(BuilderLeaderboardEntryDto::from).collect();
            match ctx.output.format {
                OutputFormat::Pretty => {
                    for dto in &dtos {
                        let tick = if dto.verified { "✓" } else { " " };
                        println!(
                            "  #{:>3}  {}{:<30}  vol {}  users {}",
                            dto.rank, tick, dto.builder, dto.volume, dto.active_users
                        );
                    }
                }
                _ => {
                    let val = serde_json::to_value(&dtos).map_err(|e| {
                        HeatError::internal("serialize", format!("Serialization failed: {e}"))
                    })?;
                    ctx.output.write_data(&val, None).map_err(io_err)?;
                }
            }
            Ok(())
        }

        DataSubcommand::BuilderVolume { period } => {
            let tp = parse_time_period(&period)?;
            let req = BuilderVolumeRequest::builder().time_period(tp).build();
            let vol = client.builder_volume(&req).await.map_err(data_err)?;
            let dtos: Vec<BuilderVolumeEntryDto> =
                vol.iter().map(BuilderVolumeEntryDto::from).collect();
            let val = serde_json::to_value(&dtos).map_err(|e| {
                HeatError::internal("serialize", format!("Serialization failed: {e}"))
            })?;
            write_json(ctx, val)
        }
    }
}

// ── address resolution ────────────────────────────────────────────────────────

/// Resolve user address: explicit flag > effective Polymarket trading wallet.
/// Parses the resolved string into an `alloy::primitives::Address`.
fn resolve_user(user: Option<String>, ctx: &Ctx) -> Result<Address, HeatError> {
    let s = match user {
        Some(u) => u,
        None => auth::resolve_pm_address(ctx, None)?,
    };
    parse_address(&s)
}

#[cfg(test)]
mod tests {
    use super::{PositionDto, ValueDto};
    use polymarket_client_sdk::data::types::{ActivityType, Side};

    // ── fmt_side ────────────────────────────────────────────────────────────

    #[test]
    fn fmt_side_produces_lowercase_strings() {
        assert_eq!(super::fmt_side(&Side::Buy), "buy");
        assert_eq!(super::fmt_side(&Side::Sell), "sell");
    }

    #[test]
    fn fmt_side_unknown_variant_is_lowercased() {
        let s = super::fmt_side(&Side::Unknown("WEIRD".to_string()));
        assert_eq!(s, s.to_lowercase(), "unknown side must be lowercased");
    }

    // ── fmt_activity_type ───────────────────────────────────────────────────

    #[test]
    fn fmt_activity_type_all_variants_lowercase() {
        let cases = [
            (ActivityType::Trade, "trade"),
            (ActivityType::Split, "split"),
            (ActivityType::Merge, "merge"),
            (ActivityType::Redeem, "redeem"),
            (ActivityType::Reward, "reward"),
            (ActivityType::Conversion, "conversion"),
            (ActivityType::Yield, "yield"),
            (ActivityType::MakerRebate, "maker_rebate"),
        ];
        for (variant, expected) in cases {
            assert_eq!(
                super::fmt_activity_type(&variant),
                expected,
                "ActivityType::{variant:?} should format as '{expected}'"
            );
        }
    }

    #[test]
    fn fmt_activity_type_unknown_is_lowercased() {
        let s = super::fmt_activity_type(&ActivityType::Unknown("CUSTOM".to_string()));
        assert_eq!(
            s,
            s.to_lowercase(),
            "unknown activity type must be lowercased"
        );
    }

    // ── parse_time_period ───────────────────────────────────────────────────

    #[test]
    fn parse_time_period_accepts_known_values() {
        assert!(super::parse_time_period("day").is_ok());
        assert!(super::parse_time_period("1d").is_ok());
        assert!(super::parse_time_period("week").is_ok());
        assert!(super::parse_time_period("1w").is_ok());
        assert!(super::parse_time_period("month").is_ok());
        assert!(super::parse_time_period("1m").is_ok());
        assert!(super::parse_time_period("all").is_ok());
    }

    #[test]
    fn parse_time_period_is_case_insensitive() {
        assert!(super::parse_time_period("DAY").is_ok());
        assert!(super::parse_time_period("Week").is_ok());
        assert!(super::parse_time_period("ALL").is_ok());
    }

    #[test]
    fn parse_time_period_rejects_unknown() {
        assert!(super::parse_time_period("yesterday").is_err());
        assert!(super::parse_time_period("").is_err());
        assert!(super::parse_time_period("5d").is_err());
    }

    // ── PositionDto serialization ────────────────────────────────────────────

    #[test]
    fn position_dto_money_fields_serialize_as_strings() {
        // Build a PositionDto directly without needing a live Position.
        let dto = PositionDto {
            proxy_wallet: "0x0000000000000000000000000000000000000001".to_string(),
            condition_id: "0x0000000000000000000000000000000000000000000000000000000000000001"
                .to_string(),
            asset: "0x1".to_string(),
            outcome: "Yes".to_string(),
            outcome_index: 0,
            size: "100.5".to_string(),
            avg_price: "0.65".to_string(),
            cur_price: "0.70".to_string(),
            current_value: "70.35".to_string(),
            initial_value: "65.325".to_string(),
            cash_pnl: "5.025".to_string(),
            percent_pnl: "7.692".to_string(),
            realized_pnl: "0".to_string(),
            total_bought: "100.5".to_string(),
            redeemable: false,
            mergeable: false,
            title: "Test Market".to_string(),
            slug: "test-market".to_string(),
            event_slug: "test-event".to_string(),
            end_date: "2025-01-01T00:00:00Z".to_string(),
            negative_risk: false,
        };

        let json = serde_json::to_value(&dto).expect("serialization must succeed");

        // Money fields must be JSON strings, not numbers, to preserve decimal precision.
        assert!(json["size"].is_string(), "size must serialize as string");
        assert!(
            json["avg_price"].is_string(),
            "avg_price must serialize as string"
        );
        assert!(
            json["cur_price"].is_string(),
            "cur_price must serialize as string"
        );
        assert!(
            json["current_value"].is_string(),
            "current_value must serialize as string"
        );
        assert!(
            json["cash_pnl"].is_string(),
            "cash_pnl must serialize as string"
        );
        assert!(
            json["percent_pnl"].is_string(),
            "percent_pnl must serialize as string"
        );

        // Boolean fields must serialize as booleans.
        assert!(
            json["redeemable"].is_boolean(),
            "redeemable must serialize as bool"
        );
        assert!(
            json["negative_risk"].is_boolean(),
            "negative_risk must serialize as bool"
        );

        // Integer fields must serialize as numbers.
        assert!(
            json["outcome_index"].is_number(),
            "outcome_index must serialize as number"
        );

        // Verify no Debug-formatted content (PascalCase struct names would indicate Debug derive).
        let raw = serde_json::to_string(&dto).unwrap();
        assert!(
            !raw.contains("PositionDto"),
            "serialized JSON must not contain struct name"
        );
    }

    #[test]
    fn position_dto_json_has_expected_keys() {
        let dto = PositionDto {
            proxy_wallet: "0xabc".to_string(),
            condition_id: "0xdef".to_string(),
            asset: "0x1".to_string(),
            outcome: "No".to_string(),
            outcome_index: 1,
            size: "50".to_string(),
            avg_price: "0.4".to_string(),
            cur_price: "0.35".to_string(),
            current_value: "17.5".to_string(),
            initial_value: "20".to_string(),
            cash_pnl: "-2.5".to_string(),
            percent_pnl: "-12.5".to_string(),
            realized_pnl: "0".to_string(),
            total_bought: "50".to_string(),
            redeemable: true,
            mergeable: true,
            title: "Another Market".to_string(),
            slug: "another-market".to_string(),
            event_slug: "another-event".to_string(),
            end_date: "2025-06-01T00:00:00Z".to_string(),
            negative_risk: true,
        };

        let json = serde_json::to_value(&dto).unwrap();
        let obj = json
            .as_object()
            .expect("PositionDto must serialize to JSON object");

        // Required keys must be present.
        for key in &[
            "proxy_wallet",
            "condition_id",
            "asset",
            "outcome",
            "outcome_index",
            "size",
            "avg_price",
            "cur_price",
            "current_value",
            "initial_value",
            "cash_pnl",
            "percent_pnl",
            "realized_pnl",
            "total_bought",
            "redeemable",
            "mergeable",
            "title",
            "slug",
            "event_slug",
            "end_date",
            "negative_risk",
        ] {
            assert!(obj.contains_key(*key), "JSON must contain key '{key}'");
        }
    }

    // ── ValueDto serialization ───────────────────────────────────────────────

    #[test]
    fn value_dto_value_field_is_string() {
        let dto = ValueDto {
            user: "0x0000000000000000000000000000000000000001".to_string(),
            value: "1234.56".to_string(),
        };

        let json = serde_json::to_value(&dto).unwrap();
        assert!(
            json["value"].is_string(),
            "value must be a JSON string, not a number"
        );
        assert_eq!(json["value"].as_str().unwrap(), "1234.56");
    }
}
