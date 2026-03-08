//! CLOB commands — trading, pricing, order management, rewards.

use alloy::primitives::{B256, U256};
use clap::Subcommand;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use heat_core::safety::DryRunPreview;
use polymarket_client_sdk::clob;
use polymarket_client_sdk::clob::types::{Interval, Side};
use polymarket_client_sdk::clob::types::request::{
    BalanceAllowanceRequest, CancelMarketOrderRequest, DeleteNotificationsRequest,
    LastTradePriceRequest, MidpointRequest, OrderBookSummaryRequest, OrdersRequest,
    PriceHistoryRequest, PriceRequest, SpreadRequest, TradesRequest, UserRewardsEarningRequest,
};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::auth;

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}

fn clob_err(e: impl std::fmt::Display) -> HeatError {
    HeatError::network("clob_request", format!("CLOB API error: {e}"))
}

// ── Heat-owned output DTOs ───────────────────────────────────────────────

#[derive(Serialize)]
struct PostOrderResult {
    order_id: String,
    success: bool,
    status: String,
    making_amount: String,
    taking_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct TradeInfo {
    id: String,
    market: String,
    side: String,
    size: String,
    price: String,
    status: String,
    outcome: String,
    match_time: String,
}

#[derive(Serialize)]
struct CancelResult {
    canceled: Vec<String>,
    not_canceled: Vec<String>,
}

#[derive(Serialize)]
struct BalanceAllowanceInfo {
    balance: String,
}

#[derive(Serialize)]
struct PriceHistoryPoint {
    timestamp: i64,
    price: String,
}

#[derive(Serialize)]
struct BookLevel {
    price: String,
    size: String,
}

#[derive(Serialize)]
struct BookInfo {
    market: String,
    asset_id: String,
    bids: Vec<BookLevel>,
    asks: Vec<BookLevel>,
    min_order_size: String,
    tick_size: String,
    neg_risk: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_trade_price: Option<String>,
}

#[derive(Serialize)]
struct TokenInfo {
    token_id: String,
    outcome: String,
    price: String,
    winner: bool,
}

#[derive(Serialize)]
struct MarketInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    condition_id: Option<String>,
    question: String,
    active: bool,
    closed: bool,
    neg_risk: bool,
    min_order_size: String,
    min_tick_size: String,
    tokens: Vec<TokenInfo>,
}

#[derive(Serialize)]
struct MarketsPage {
    markets: Vec<MarketInfo>,
    next_cursor: String,
}

#[derive(Serialize)]
struct SimplifiedTokenInfo {
    token_id: String,
    outcome: String,
    price: String,
    winner: bool,
}

#[derive(Serialize)]
struct SimplifiedMarketInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    condition_id: Option<String>,
    active: bool,
    closed: bool,
    archived: bool,
    accepting_orders: bool,
    tokens: Vec<SimplifiedTokenInfo>,
}

#[derive(Serialize)]
struct SimplifiedMarketsPage {
    markets: Vec<SimplifiedMarketInfo>,
    next_cursor: String,
}

#[derive(Serialize)]
struct EarningEntry {
    date: String,
    condition_id: String,
    asset_address: String,
    maker_address: String,
    earnings: String,
    asset_rate: String,
}

#[derive(Serialize)]
struct EarningsPage {
    earnings: Vec<EarningEntry>,
    next_cursor: String,
}

#[derive(Serialize)]
struct TotalEarningEntry {
    date: String,
    asset_address: String,
    maker_address: String,
    earnings: String,
    asset_rate: String,
}

#[derive(Serialize)]
struct RewardsConfigInfo {
    asset_address: String,
    start_date: String,
    end_date: String,
    rate_per_day: String,
    total_rewards: String,
}

#[derive(Serialize)]
struct CurrentRewardEntry {
    condition_id: String,
    rewards_max_spread: String,
    rewards_min_size: String,
    rewards_config: Vec<RewardsConfigInfo>,
}

#[derive(Serialize)]
struct CurrentRewardsPage {
    rewards: Vec<CurrentRewardEntry>,
    next_cursor: String,
}

#[derive(Serialize)]
struct MarketRewardsConfigInfo {
    id: String,
    asset_address: String,
    start_date: String,
    end_date: String,
    rate_per_day: String,
    total_rewards: String,
    total_days: String,
}

#[derive(Serialize)]
struct RawRewardEntry {
    condition_id: String,
    question: String,
    market_slug: String,
    rewards_max_spread: String,
    rewards_min_size: String,
    rewards_config: Vec<MarketRewardsConfigInfo>,
}

#[derive(Serialize)]
struct RawRewardsPage {
    rewards: Vec<RawRewardEntry>,
    next_cursor: String,
}

#[derive(Serialize)]
struct UserEarningsConfigEntry {
    condition_id: String,
    question: String,
    market_slug: String,
    rewards_max_spread: String,
    rewards_min_size: String,
    earning_percentage: String,
    maker_address: String,
    tokens: Vec<TokenInfo>,
}

// ── Commands ─────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ClobSubcommand {
    // ── Pricing (no auth) ────────────────────────────────────────────
    /// Get price for a token
    Price {
        token_id: String,
        #[arg(long)]
        side: String,
    },
    /// Get midpoint price
    Midpoint { token_id: String },
    /// Get spread
    Spread {
        token_id: String,
        #[arg(long)]
        side: Option<String>,
    },
    /// Get order book
    Book { token_id: String },
    /// Get last trade price
    LastTradePrice { token_id: String },
    /// Get price history
    PriceHistory {
        market: String,
        #[arg(long, default_value = "1d")]
        range: String,
    },

    // ── Market info (no auth) ────────────────────────────────────────
    /// Get market info by condition ID
    Market { condition_id: String },
    /// List CLOB markets
    Markets {
        #[arg(long)]
        cursor: Option<String>,
    },
    /// List sampling markets
    SamplingMarkets {
        #[arg(long)]
        cursor: Option<String>,
    },
    /// List simplified markets
    SimplifiedMarkets {
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Get tick size for a token
    TickSize { token_id: String },
    /// Get fee rate in basis points
    FeeRate { token_id: String },
    /// Check if market uses neg risk
    NegRisk { token_id: String },
    /// Get server time
    ServerTime,
    /// Check geoblock status
    Geoblock,
    /// Check CLOB health
    Ok,

    // ── Orders (auth required) ───────────────────────────────────────
    /// Place a limit order
    LimitOrder {
        token_id: String,
        #[arg(long)]
        side: String,
        #[arg(long)]
        price: Decimal,
        #[arg(long)]
        size: Decimal,
        #[arg(long, default_value = "gtc")]
        order_type: String,
        #[arg(long)]
        post_only: bool,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Place a market order
    MarketOrder {
        token_id: String,
        #[arg(long)]
        side: String,
        #[arg(long)]
        amount: Decimal,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Get a specific order
    Order {
        order_id: String,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// List open orders
    Orders {
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        asset_id: Option<String>,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// List trades
    Trades {
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        asset_id: Option<String>,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Cancel a specific order
    CancelOrder {
        order_id: String,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Cancel orders for a market
    CancelMarketOrders {
        #[arg(long)]
        market: Option<String>,
        #[arg(long)]
        asset_id: Option<String>,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Cancel all open orders
    CancelAll {
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Check balance and allowance
    BalanceAllowance {
        #[arg(long)]
        asset_type: String,
        #[arg(long)]
        token_id: Option<String>,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Update balance allowance
    UpdateBalanceAllowance {
        #[arg(long)]
        sig_type: Option<String>,
    },

    // ── Notifications ────────────────────────────────────────────────
    /// List notifications
    Notifications {
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Delete notifications
    DeleteNotifications {
        /// Notification IDs (comma-separated)
        ids: String,
        #[arg(long)]
        sig_type: Option<String>,
    },

    // ── API keys ─────────────────────────────────────────────────────
    /// List API keys
    ApiKeys {
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Create or derive an API key
    CreateApiKey {
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Delete current API key
    DeleteApiKey {
        #[arg(long)]
        sig_type: Option<String>,
    },

    // ── Rewards ──────────────────────────────────────────────────────
    /// Get earnings for a specific day
    Earnings {
        /// Date (YYYY-MM-DD)
        date: String,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Get total earnings for a day
    TotalEarnings {
        date: String,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Get reward percentages
    RewardPercentages {
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Get current rewards
    CurrentRewards {
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Get raw rewards for a market
    RawRewards {
        condition_id: String,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Check if an order is scoring
    IsOrderScoring {
        order_id: String,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Check if multiple orders are scoring
    AreOrdersScoring {
        /// Order IDs (comma-separated)
        order_ids: String,
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Check closed-only mode
    ClosedOnlyMode {
        #[arg(long)]
        sig_type: Option<String>,
    },
    /// Get user earnings and markets config for a day
    UserEarningsConfig {
        date: String,
        #[arg(long)]
        sig_type: Option<String>,
    },
}

fn parse_side(s: &str) -> Result<Side, HeatError> {
    match s.to_lowercase().as_str() {
        "buy" => Ok(Side::Buy),
        "sell" => Ok(Side::Sell),
        _ => Err(HeatError::validation("invalid_side", format!("Invalid side: {s}"))
            .with_hint("Valid sides: buy, sell")),
    }
}

fn parse_interval(s: &str) -> Result<Interval, HeatError> {
    match s.to_lowercase().as_str() {
        "1m" => Ok(Interval::OneMinute),
        "1h" => Ok(Interval::OneHour),
        "6h" => Ok(Interval::SixHours),
        "1d" => Ok(Interval::OneDay),
        "1w" => Ok(Interval::OneWeek),
        "max" => Ok(Interval::Max),
        _ => Err(HeatError::validation("invalid_interval", format!("Invalid interval: {s}"))
            .with_hint("Valid intervals: 1m, 1h, 6h, 1d, 1w, max")),
    }
}

fn parse_order_type(s: &str) -> Result<clob::types::OrderType, HeatError> {
    match s.to_lowercase().as_str() {
        "gtc" => Ok(clob::types::OrderType::GTC),
        "fok" => Ok(clob::types::OrderType::FOK),
        "gtd" => Ok(clob::types::OrderType::GTD),
        "fak" => Ok(clob::types::OrderType::FAK),
        _ => Err(HeatError::validation("invalid_order_type", format!("Invalid order type: {s}"))
            .with_hint("Valid types: gtc, fok, gtd, fak")),
    }
}

fn parse_date(s: &str) -> Result<chrono::NaiveDate, HeatError> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
        HeatError::validation("invalid_date", format!("Invalid date '{s}': {e}"))
            .with_hint("Expected format: YYYY-MM-DD")
    })
}

fn parse_u256(s: &str) -> Result<U256, HeatError> {
    s.parse().map_err(|_| {
        HeatError::validation("invalid_u256", format!("Invalid U256 value: {s}"))
    })
}

fn parse_b256(s: &str) -> Result<B256, HeatError> {
    s.parse().map_err(|_| {
        HeatError::validation("invalid_b256", format!("Invalid B256 value: {s}"))
    })
}

/// Map Side enum to a stable lowercase string.
fn side_str(side: &Side) -> &'static str {
    match side {
        Side::Buy => "buy",
        Side::Sell => "sell",
        Side::Unknown => "unknown",
        _ => "unknown",
    }
}

/// Map OrderStatusType to a stable lowercase string.
fn order_status_str(status: &clob::types::OrderStatusType) -> String {
    use clob::types::OrderStatusType;
    match status {
        OrderStatusType::Live => "live".to_owned(),
        OrderStatusType::Matched => "matched".to_owned(),
        OrderStatusType::Canceled => "canceled".to_owned(),
        OrderStatusType::Delayed => "delayed".to_owned(),
        OrderStatusType::Unmatched => "unmatched".to_owned(),
        OrderStatusType::Unknown(s) => s.to_lowercase(),
        _ => "unknown".to_owned(),
    }
}

/// Map TradeStatusType to a stable lowercase string.
fn trade_status_str(status: &clob::types::TradeStatusType) -> String {
    use clob::types::TradeStatusType;
    match status {
        TradeStatusType::Matched => "matched".to_owned(),
        TradeStatusType::Mined => "mined".to_owned(),
        TradeStatusType::Confirmed => "confirmed".to_owned(),
        TradeStatusType::Retrying => "retrying".to_owned(),
        TradeStatusType::Failed => "failed".to_owned(),
        TradeStatusType::Unknown(s) => s.to_lowercase(),
        _ => "unknown".to_owned(),
    }
}

/// Helper: output a simple JSON value
fn write_json(ctx: &Ctx, val: serde_json::Value) -> Result<(), HeatError> {
    ctx.output.write_data(&val, None).map_err(io_err)
}

pub async fn run(sub: ClobSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    match sub {
        // ── Unauthenticated pricing ──────────────────────────────────
        ClobSubcommand::Price { token_id, side } => {
            let side = parse_side(&side)?;
            let client = clob::Client::default();
            let req = PriceRequest::builder().token_id(parse_u256(&token_id)?).side(side).build();
            let resp = client.price(&req).await.map_err(clob_err)?;
            match ctx.output.format {
                OutputFormat::Pretty => println!("{}", resp.price),
                _ => write_json(ctx, serde_json::json!({ "price": resp.price.to_string() }))?,
            }
            Ok(())
        }
        ClobSubcommand::Midpoint { token_id } => {
            let client = clob::Client::default();
            let req = MidpointRequest::builder().token_id(parse_u256(&token_id)?).build();
            let resp = client.midpoint(&req).await.map_err(clob_err)?;
            match ctx.output.format {
                OutputFormat::Pretty => println!("{}", resp.mid),
                _ => write_json(ctx, serde_json::json!({ "midpoint": resp.mid.to_string() }))?,
            }
            Ok(())
        }
        ClobSubcommand::Spread { token_id, side } => {
            let client = clob::Client::default();
            let tid = parse_u256(&token_id)?;
            let parsed_side = side.as_ref().map(|s| parse_side(s)).transpose()?;
            let req = if let Some(s) = parsed_side {
                SpreadRequest::builder().token_id(tid).side(s).build()
            } else {
                SpreadRequest::builder().token_id(tid).build()
            };
            let resp = client.spread(&req).await.map_err(clob_err)?;
            match ctx.output.format {
                OutputFormat::Pretty => println!("{}", resp.spread),
                _ => write_json(ctx, serde_json::json!({ "spread": resp.spread.to_string() }))?,
            }
            Ok(())
        }
        ClobSubcommand::Book { token_id } => {
            let client = clob::Client::default();
            let req = OrderBookSummaryRequest::builder().token_id(parse_u256(&token_id)?).build();
            let resp = client.order_book(&req).await.map_err(clob_err)?;
            let bids: Vec<BookLevel> = resp.bids.iter()
                .map(|b| BookLevel { price: b.price.to_string(), size: b.size.to_string() })
                .collect();
            let asks: Vec<BookLevel> = resp.asks.iter()
                .map(|a| BookLevel { price: a.price.to_string(), size: a.size.to_string() })
                .collect();
            let info = BookInfo {
                market: format!("{}", resp.market),
                asset_id: resp.asset_id.to_string(),
                bids,
                asks,
                min_order_size: resp.min_order_size.to_string(),
                tick_size: resp.tick_size.as_decimal().to_string(),
                neg_risk: resp.neg_risk,
                last_trade_price: resp.last_trade_price.map(|p| p.to_string()),
            };
            match ctx.output.format {
                OutputFormat::Pretty => {
                    println!("Market:         {}", info.market);
                    println!("Asset:          {}", info.asset_id);
                    println!("Bids:           {}", info.bids.len());
                    println!("Asks:           {}", info.asks.len());
                    println!("Min order size: {}", info.min_order_size);
                    println!("Tick size:      {}", info.tick_size);
                    println!("Neg risk:       {}", info.neg_risk);
                    if let Some(ref ltp) = info.last_trade_price {
                        println!("Last trade:     {ltp}");
                    }
                }
                _ => { ctx.output.write_data(&info, None).map_err(io_err)?; }
            }
            Ok(())
        }
        ClobSubcommand::LastTradePrice { token_id } => {
            let client = clob::Client::default();
            let req = LastTradePriceRequest::builder().token_id(parse_u256(&token_id)?).build();
            let resp = client.last_trade_price(&req).await.map_err(clob_err)?;
            let side = side_str(&resp.side).to_owned();
            match ctx.output.format {
                OutputFormat::Pretty => println!("{} ({})", resp.price, side),
                _ => write_json(ctx, serde_json::json!({
                    "price": resp.price.to_string(),
                    "side": side,
                }))?,
            }
            Ok(())
        }
        ClobSubcommand::PriceHistory { market, range } => {
            let client = clob::Client::default();
            let interval = parse_interval(&range)?;
            let req = PriceHistoryRequest::builder()
                .market(parse_u256(&market)?)
                .time_range(interval)
                .build();
            let resp = client.price_history(&req).await.map_err(clob_err)?;
            let points: Vec<PriceHistoryPoint> = resp
                .history
                .iter()
                .map(|p| PriceHistoryPoint {
                    timestamp: p.t,
                    price: p.p.to_string(),
                })
                .collect();
            ctx.output.write_data(&points, None).map_err(io_err)
        }

        // ── Market info (no auth) ────────────────────────────────────
        ClobSubcommand::Market { condition_id } => {
            let client = clob::Client::default();
            let resp = client.market(&condition_id).await.map_err(clob_err)?;
            let tokens: Vec<TokenInfo> = resp.tokens.iter()
                .map(|t| TokenInfo {
                    token_id: t.token_id.to_string(),
                    outcome: t.outcome.clone(),
                    price: t.price.to_string(),
                    winner: t.winner,
                })
                .collect();
            let info = MarketInfo {
                condition_id: resp.condition_id.map(|id| format!("{id}")),
                question: resp.question.clone(),
                active: resp.active,
                closed: resp.closed,
                neg_risk: resp.neg_risk,
                min_order_size: resp.minimum_order_size.to_string(),
                min_tick_size: resp.minimum_tick_size.to_string(),
                tokens,
            };
            match ctx.output.format {
                OutputFormat::Pretty => {
                    println!("Active:           {}", info.active);
                    println!("Closed:           {}", info.closed);
                    println!("Neg risk:         {}", info.neg_risk);
                    println!("Min order size:   {}", info.min_order_size);
                    println!("Min tick size:    {}", info.min_tick_size);
                    println!("Question:         {}", info.question);
                    println!("Tokens:           {}", info.tokens.len());
                }
                _ => { ctx.output.write_data(&info, None).map_err(io_err)?; }
            }
            Ok(())
        }
        ClobSubcommand::Markets { cursor } => {
            let client = clob::Client::default();
            let resp = client.markets(cursor).await.map_err(clob_err)?;
            let markets: Vec<MarketInfo> = resp.data.iter()
                .map(|m| {
                    let tokens: Vec<TokenInfo> = m.tokens.iter()
                        .map(|t| TokenInfo {
                            token_id: t.token_id.to_string(),
                            outcome: t.outcome.clone(),
                            price: t.price.to_string(),
                            winner: t.winner,
                        })
                        .collect();
                    MarketInfo {
                        condition_id: m.condition_id.map(|id| format!("{id}")),
                        question: m.question.clone(),
                        active: m.active,
                        closed: m.closed,
                        neg_risk: m.neg_risk,
                        min_order_size: m.minimum_order_size.to_string(),
                        min_tick_size: m.minimum_tick_size.to_string(),
                        tokens,
                    }
                })
                .collect();
            let page = MarketsPage {
                markets,
                next_cursor: resp.next_cursor.clone(),
            };
            ctx.output.write_data(&page, None).map_err(io_err)
        }
        ClobSubcommand::SamplingMarkets { cursor } => {
            let client = clob::Client::default();
            let resp = client.sampling_markets(cursor).await.map_err(clob_err)?;
            let markets: Vec<MarketInfo> = resp.data.iter()
                .map(|m| {
                    let tokens: Vec<TokenInfo> = m.tokens.iter()
                        .map(|t| TokenInfo {
                            token_id: t.token_id.to_string(),
                            outcome: t.outcome.clone(),
                            price: t.price.to_string(),
                            winner: t.winner,
                        })
                        .collect();
                    MarketInfo {
                        condition_id: m.condition_id.map(|id| format!("{id}")),
                        question: m.question.clone(),
                        active: m.active,
                        closed: m.closed,
                        neg_risk: m.neg_risk,
                        min_order_size: m.minimum_order_size.to_string(),
                        min_tick_size: m.minimum_tick_size.to_string(),
                        tokens,
                    }
                })
                .collect();
            let page = MarketsPage {
                markets,
                next_cursor: resp.next_cursor.clone(),
            };
            ctx.output.write_data(&page, None).map_err(io_err)
        }
        ClobSubcommand::SimplifiedMarkets { cursor } => {
            let client = clob::Client::default();
            let resp = client.simplified_markets(cursor).await.map_err(clob_err)?;
            let markets: Vec<SimplifiedMarketInfo> = resp.data.iter()
                .map(|m| {
                    let tokens: Vec<SimplifiedTokenInfo> = m.tokens.iter()
                        .map(|t| SimplifiedTokenInfo {
                            token_id: t.token_id.to_string(),
                            outcome: t.outcome.clone(),
                            price: t.price.to_string(),
                            winner: t.winner,
                        })
                        .collect();
                    SimplifiedMarketInfo {
                        condition_id: m.condition_id.map(|id| format!("{id}")),
                        active: m.active,
                        closed: m.closed,
                        archived: m.archived,
                        accepting_orders: m.accepting_orders,
                        tokens,
                    }
                })
                .collect();
            let page = SimplifiedMarketsPage {
                markets,
                next_cursor: resp.next_cursor.clone(),
            };
            ctx.output.write_data(&page, None).map_err(io_err)
        }
        ClobSubcommand::TickSize { token_id } => {
            let client = clob::Client::default();
            let id = token_id.parse().map_err(|_| {
                HeatError::validation("invalid_token_id", "Token ID must be a valid U256")
            })?;
            let resp = client.tick_size(id).await.map_err(clob_err)?;
            let tick_str = resp.minimum_tick_size.as_decimal().to_string();
            match ctx.output.format {
                OutputFormat::Pretty => println!("{tick_str}"),
                _ => write_json(ctx, serde_json::json!({ "tick_size": tick_str }))?,
            }
            Ok(())
        }
        ClobSubcommand::FeeRate { token_id } => {
            let client = clob::Client::default();
            let id = token_id.parse().map_err(|_| {
                HeatError::validation("invalid_token_id", "Token ID must be a valid U256")
            })?;
            let resp = client.fee_rate_bps(id).await.map_err(clob_err)?;
            match ctx.output.format {
                OutputFormat::Pretty => println!("{} bps", resp.base_fee),
                _ => write_json(ctx, serde_json::json!({ "fee_rate_bps": resp.base_fee }))?,
            }
            Ok(())
        }
        ClobSubcommand::NegRisk { token_id } => {
            let client = clob::Client::default();
            let id = token_id.parse().map_err(|_| {
                HeatError::validation("invalid_token_id", "Token ID must be a valid U256")
            })?;
            let resp = client.neg_risk(id).await.map_err(clob_err)?;
            match ctx.output.format {
                OutputFormat::Pretty => println!("{}", resp.neg_risk),
                _ => write_json(ctx, serde_json::json!({ "neg_risk": resp.neg_risk }))?,
            }
            Ok(())
        }
        ClobSubcommand::ServerTime => {
            let client = clob::Client::default();
            let t = client.server_time().await.map_err(clob_err)?;
            match ctx.output.format {
                OutputFormat::Pretty => println!("{t}"),
                _ => write_json(ctx, serde_json::json!({ "server_time": t }))?,
            }
            Ok(())
        }
        ClobSubcommand::Geoblock => {
            let client = clob::Client::default();
            let resp = client.check_geoblock().await.map_err(clob_err)?;
            let info = serde_json::json!({
                "blocked": resp.blocked,
                "ip": resp.ip,
                "country": resp.country,
                "region": resp.region,
            });
            match ctx.output.format {
                OutputFormat::Pretty => {
                    println!("Blocked: {}", resp.blocked);
                    println!("IP:      {}", resp.ip);
                    println!("Country: {}", resp.country);
                }
                _ => write_json(ctx, info)?,
            }
            Ok(())
        }
        ClobSubcommand::Ok => {
            let client = clob::Client::default();
            let ok = client.ok().await.map_err(clob_err)?;
            match ctx.output.format {
                OutputFormat::Pretty => println!("CLOB: {ok}"),
                _ => write_json(ctx, serde_json::json!({ "ok": ok }))?,
            }
            Ok(())
        }

        // ── Authenticated order commands ─────────────────────────────
        ClobSubcommand::LimitOrder {
            token_id, side, price, size, order_type, post_only, sig_type,
        } => {
            let side = parse_side(&side)?;
            let ot = parse_order_type(&order_type)?;

            if ctx.dry_run {
                DryRunPreview::new("pm", "clob limit-order")
                    .param("token_id", &token_id)
                    .param("side", side_str(&side))
                    .param("price", &price.to_string())
                    .param("size", &size.to_string())
                    .param("order_type", &order_type)
                    .param("post_only", &post_only.to_string())
                    .display();
                return Ok(());
            }

            ctx.confirm_dangerous(&format!(
                "place {} limit order: {size} @ {price} on {token_id}",
                side_str(&side)
            ))?;

            let signer = auth::resolve_signer(ctx)?;
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let signable = client
                .limit_order()
                .token_id(parse_u256(&token_id)?)
                .side(side)
                .price(price)
                .size(size)
                .order_type(ot)
                .post_only(post_only)
                .build()
                .await
                .map_err(clob_err)?;
            let signed = client.sign(&signer, signable).await.map_err(clob_err)?;
            let resp = client.post_order(signed).await.map_err(clob_err)?;

            let result = PostOrderResult {
                order_id: resp.order_id.clone(),
                success: resp.success,
                status: order_status_str(&resp.status),
                making_amount: resp.making_amount.to_string(),
                taking_amount: resp.taking_amount.to_string(),
                error: resp.error_msg.clone(),
            };

            match ctx.output.format {
                OutputFormat::Pretty => {
                    if result.success {
                        println!("Order placed: {}", result.order_id);
                    } else {
                        println!("Order failed: {}", result.error.as_deref().unwrap_or("unknown"));
                    }
                }
                _ => { ctx.output.write_data(&result, None).map_err(io_err)?; }
            }
            Ok(())
        }
        ClobSubcommand::MarketOrder {
            token_id, side, amount, sig_type,
        } => {
            let side = parse_side(&side)?;

            if ctx.dry_run {
                DryRunPreview::new("pm", "clob market-order")
                    .param("token_id", &token_id)
                    .param("side", side_str(&side))
                    .param("amount_usdc", &amount.to_string())
                    .display();
                return Ok(());
            }

            ctx.confirm_dangerous(&format!(
                "place {} market order: ${amount} on {token_id}",
                side_str(&side)
            ))?;

            let signer = auth::resolve_signer(ctx)?;
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let signable = client
                .market_order()
                .token_id(parse_u256(&token_id)?)
                .side(side)
                .amount(clob::types::Amount::usdc(amount).map_err(clob_err)?)
                .build()
                .await
                .map_err(clob_err)?;
            let signed = client.sign(&signer, signable).await.map_err(clob_err)?;
            let resp = client.post_order(signed).await.map_err(clob_err)?;

            let result = PostOrderResult {
                order_id: resp.order_id.clone(),
                success: resp.success,
                status: order_status_str(&resp.status),
                making_amount: resp.making_amount.to_string(),
                taking_amount: resp.taking_amount.to_string(),
                error: resp.error_msg.clone(),
            };
            ctx.output.write_data(&result, None).map_err(io_err)
        }
        ClobSubcommand::Order { order_id, sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client.order(&order_id).await.map_err(clob_err)?;
            write_json(ctx, serde_json::json!({
                "order_id": resp.id,
                "status": order_status_str(&resp.status),
                "side": side_str(&resp.side),
                "original_size": resp.original_size.to_string(),
                "price": resp.price.to_string(),
                "size_matched": resp.size_matched.to_string(),
                "market": format!("{}", resp.market),
            }))
        }
        ClobSubcommand::Orders { market, asset_id, sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let mut req = OrdersRequest::default();
            if let Some(m) = &market {
                req.market = Some(parse_b256(m)?);
            }
            if let Some(a) = &asset_id {
                req.asset_id = Some(parse_u256(a)?);
            }
            let orders_page = client.orders(&req, None).await.map_err(clob_err)?;
            let infos: Vec<serde_json::Value> = orders_page.data
                .iter()
                .map(|o| serde_json::json!({
                    "order_id": o.id,
                    "status": order_status_str(&o.status),
                    "side": side_str(&o.side),
                    "original_size": o.original_size.to_string(),
                    "price": o.price.to_string(),
                    "size_matched": o.size_matched.to_string(),
                    "market": format!("{}", o.market),
                }))
                .collect();
            ctx.output.write_data(&infos, None).map_err(io_err)
        }
        ClobSubcommand::Trades { market, asset_id, sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let mut req = TradesRequest::default();
            if let Some(m) = &market {
                req.market = Some(parse_b256(m)?);
            }
            if let Some(a) = &asset_id {
                req.asset_id = Some(parse_u256(a)?);
            }
            let trades_page = client.trades(&req, None).await.map_err(clob_err)?;
            let infos: Vec<TradeInfo> = trades_page.data
                .iter()
                .map(|t| TradeInfo {
                    id: t.id.clone(),
                    market: format!("{}", t.market),
                    side: side_str(&t.side).to_owned(),
                    size: t.size.to_string(),
                    price: t.price.to_string(),
                    status: trade_status_str(&t.status),
                    outcome: t.outcome.clone(),
                    match_time: t.match_time.to_rfc3339(),
                })
                .collect();
            ctx.output.write_data(&infos, None).map_err(io_err)
        }
        ClobSubcommand::CancelOrder { order_id, sig_type } => {
            if ctx.dry_run {
                DryRunPreview::new("pm", "clob cancel-order")
                    .param("order_id", &order_id)
                    .display();
                return Ok(());
            }
            ctx.confirm_dangerous(&format!("cancel order {order_id}"))?;
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client.cancel_order(&order_id).await.map_err(clob_err)?;
            let result = CancelResult {
                canceled: resp.canceled.clone(),
                not_canceled: resp.not_canceled.keys().cloned().collect(),
            };
            ctx.output.write_data(&result, None).map_err(io_err)
        }
        ClobSubcommand::CancelMarketOrders { market, asset_id, sig_type } => {
            if ctx.dry_run {
                DryRunPreview::new("pm", "clob cancel-market-orders")
                    .param("market", market.as_deref().unwrap_or("all"))
                    .display();
                return Ok(());
            }
            ctx.confirm_dangerous("cancel market orders")?;
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let mut req = CancelMarketOrderRequest::default();
            if let Some(m) = &market {
                req.market = Some(parse_b256(m)?);
            }
            if let Some(a) = &asset_id {
                req.asset_id = Some(parse_u256(a)?);
            }
            let resp = client.cancel_market_orders(&req).await.map_err(clob_err)?;
            let result = CancelResult {
                canceled: resp.canceled.clone(),
                not_canceled: resp.not_canceled.keys().cloned().collect(),
            };
            ctx.output.write_data(&result, None).map_err(io_err)
        }
        ClobSubcommand::CancelAll { sig_type } => {
            if ctx.dry_run {
                DryRunPreview::new("pm", "clob cancel-all").display();
                return Ok(());
            }
            ctx.confirm_dangerous("cancel ALL open orders")?;
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client.cancel_all_orders().await.map_err(clob_err)?;
            let result = CancelResult {
                canceled: resp.canceled.clone(),
                not_canceled: resp.not_canceled.keys().cloned().collect(),
            };
            ctx.output.write_data(&result, None).map_err(io_err)
        }
        ClobSubcommand::BalanceAllowance { asset_type, token_id, sig_type } => {
            let at = match asset_type.to_lowercase().as_str() {
                "collateral" => clob::types::AssetType::Collateral,
                "conditional" => clob::types::AssetType::Conditional,
                _ => {
                    return Err(HeatError::validation(
                        "invalid_asset_type",
                        format!("Invalid asset type: {asset_type}"),
                    )
                    .with_hint("Valid types: collateral, conditional"));
                }
            };
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let mut req = BalanceAllowanceRequest::default();
            req.asset_type = at;
            if let Some(tid) = &token_id {
                req.token_id = Some(parse_u256(tid)?);
            }
            let resp = client.balance_allowance(req).await.map_err(clob_err)?;
            let info = BalanceAllowanceInfo {
                balance: resp.balance.to_string(),
            };
            match ctx.output.format {
                OutputFormat::Pretty => println!("Balance: {}", info.balance),
                _ => { ctx.output.write_data(&info, None).map_err(io_err)?; }
            }
            Ok(())
        }
        ClobSubcommand::UpdateBalanceAllowance { sig_type } => {
            if ctx.dry_run {
                DryRunPreview::new("pm", "clob update-balance-allowance").display();
                return Ok(());
            }
            ctx.confirm_dangerous("update balance allowance")?;
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            client
                .update_balance_allowance(
                    BalanceAllowanceRequest::builder()
                        .asset_type(clob::types::AssetType::Collateral)
                        .build(),
                )
                .await
                .map_err(clob_err)?;
            write_json(ctx, serde_json::json!({ "success": true }))
        }

        // ── Notifications ────────────────────────────────────────────
        ClobSubcommand::Notifications { sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let notifs = client.notifications().await.map_err(clob_err)?;
            let infos: Vec<serde_json::Value> = notifs
                .iter()
                .map(|n| serde_json::json!({
                    "type": n.r#type,
                    "payload_event": n.payload.event_slug,
                }))
                .collect();
            ctx.output.write_data(&infos, None).map_err(io_err)
        }
        ClobSubcommand::DeleteNotifications { ids, sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let id_list: Vec<String> = ids.split(',').map(|s| s.trim().to_string()).collect();
            let req = DeleteNotificationsRequest::builder()
                .notification_ids(id_list)
                .build();
            client.delete_notifications(&req).await.map_err(clob_err)?;
            write_json(ctx, serde_json::json!({ "deleted": true }))
        }

        // ── API keys ─────────────────────────────────────────────────
        ClobSubcommand::ApiKeys { sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            // ApiKeysResponse.keys is private with no public accessor in the SDK.
            // We confirm the request succeeded; key data is not accessible.
            client.api_keys().await.map_err(clob_err)?;
            write_json(ctx, serde_json::json!({ "success": true }))
        }
        ClobSubcommand::CreateApiKey { sig_type: _ } => {
            let signer = auth::resolve_signer(ctx)?;
            let client = clob::Client::default();
            let creds = client
                .create_or_derive_api_key(&signer, None)
                .await
                .map_err(clob_err)?;
            write_json(ctx, serde_json::json!({
                "key": creds.key().to_string(),
            }))
        }
        ClobSubcommand::DeleteApiKey { sig_type } => {
            if ctx.dry_run {
                DryRunPreview::new("pm", "clob delete-api-key").display();
                return Ok(());
            }
            ctx.confirm_dangerous("delete API key")?;
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            client.delete_api_key().await.map_err(clob_err)?;
            write_json(ctx, serde_json::json!({ "deleted": true }))
        }

        // ── Rewards ──────────────────────────────────────────────────
        ClobSubcommand::Earnings { date, cursor, sig_type } => {
            let d = parse_date(&date)?;
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client.earnings_for_user_for_day(d, cursor).await.map_err(clob_err)?;
            let earnings: Vec<EarningEntry> = resp.data.iter()
                .map(|e| EarningEntry {
                    date: e.date.to_string(),
                    condition_id: format!("{}", e.condition_id),
                    asset_address: format!("{}", e.asset_address),
                    maker_address: format!("{}", e.maker_address),
                    earnings: e.earnings.to_string(),
                    asset_rate: e.asset_rate.to_string(),
                })
                .collect();
            let page = EarningsPage {
                earnings,
                next_cursor: resp.next_cursor.clone(),
            };
            ctx.output.write_data(&page, None).map_err(io_err)
        }
        ClobSubcommand::TotalEarnings { date, sig_type } => {
            let d = parse_date(&date)?;
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client.total_earnings_for_user_for_day(d).await.map_err(clob_err)?;
            let earnings: Vec<TotalEarningEntry> = resp.iter()
                .map(|e| TotalEarningEntry {
                    date: e.date.to_string(),
                    asset_address: format!("{}", e.asset_address),
                    maker_address: format!("{}", e.maker_address),
                    earnings: e.earnings.to_string(),
                    asset_rate: e.asset_rate.to_string(),
                })
                .collect();
            ctx.output.write_data(&earnings, None).map_err(io_err)
        }
        ClobSubcommand::RewardPercentages { sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client.reward_percentages().await.map_err(clob_err)?;
            // HashMap<String, Decimal> — convert Decimal values to strings for stable output.
            let mapped: serde_json::Map<String, serde_json::Value> = resp
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.to_string())))
                .collect();
            write_json(ctx, serde_json::json!({ "reward_percentages": mapped }))
        }
        ClobSubcommand::CurrentRewards { cursor, sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client.current_rewards(cursor).await.map_err(clob_err)?;
            let rewards: Vec<CurrentRewardEntry> = resp.data.iter()
                .map(|r| {
                    let config: Vec<RewardsConfigInfo> = r.rewards_config.iter()
                        .map(|c| RewardsConfigInfo {
                            asset_address: format!("{}", c.asset_address),
                            start_date: c.start_date.to_string(),
                            end_date: c.end_date.to_string(),
                            rate_per_day: c.rate_per_day.to_string(),
                            total_rewards: c.total_rewards.to_string(),
                        })
                        .collect();
                    CurrentRewardEntry {
                        condition_id: format!("{}", r.condition_id),
                        rewards_max_spread: r.rewards_max_spread.to_string(),
                        rewards_min_size: r.rewards_min_size.to_string(),
                        rewards_config: config,
                    }
                })
                .collect();
            let page = CurrentRewardsPage {
                rewards,
                next_cursor: resp.next_cursor.clone(),
            };
            ctx.output.write_data(&page, None).map_err(io_err)
        }
        ClobSubcommand::RawRewards { condition_id, cursor, sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client
                .raw_rewards_for_market(&condition_id, cursor)
                .await
                .map_err(clob_err)?;
            let rewards: Vec<RawRewardEntry> = resp.data.iter()
                .map(|r| {
                    let config: Vec<MarketRewardsConfigInfo> = r.rewards_config.iter()
                        .map(|c| MarketRewardsConfigInfo {
                            id: c.id.clone(),
                            asset_address: format!("{}", c.asset_address),
                            start_date: c.start_date.to_string(),
                            end_date: c.end_date.to_string(),
                            rate_per_day: c.rate_per_day.to_string(),
                            total_rewards: c.total_rewards.to_string(),
                            total_days: c.total_days.to_string(),
                        })
                        .collect();
                    RawRewardEntry {
                        condition_id: format!("{}", r.condition_id),
                        question: r.question.clone(),
                        market_slug: r.market_slug.clone(),
                        rewards_max_spread: r.rewards_max_spread.to_string(),
                        rewards_min_size: r.rewards_min_size.to_string(),
                        rewards_config: config,
                    }
                })
                .collect();
            let page = RawRewardsPage {
                rewards,
                next_cursor: resp.next_cursor.clone(),
            };
            ctx.output.write_data(&page, None).map_err(io_err)
        }
        ClobSubcommand::IsOrderScoring { order_id, sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client.is_order_scoring(&order_id).await.map_err(clob_err)?;
            write_json(ctx, serde_json::json!({ "scoring": resp.scoring }))
        }
        ClobSubcommand::AreOrdersScoring { order_ids, sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let ids: Vec<&str> = order_ids.split(',').map(|s| s.trim()).collect();
            let resp = client.are_orders_scoring(&ids).await.map_err(clob_err)?;
            // HashMap<String, bool> — already a stable type, emit directly.
            let mapped: serde_json::Map<String, serde_json::Value> = resp
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::Bool(*v)))
                .collect();
            write_json(ctx, serde_json::json!({ "scoring": mapped }))
        }
        ClobSubcommand::ClosedOnlyMode { sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let resp = client.closed_only_mode().await.map_err(clob_err)?;
            write_json(ctx, serde_json::json!({ "closed_only": resp.closed_only }))
        }
        ClobSubcommand::UserEarningsConfig { date, sig_type } => {
            let client = auth::authenticated_clob_client(ctx, sig_type.as_deref()).await?;
            let d = parse_date(&date)?;
            let req = UserRewardsEarningRequest::builder().date(d).build();
            let resp = client
                .user_earnings_and_markets_config(&req, None)
                .await
                .map_err(clob_err)?;
            let entries: Vec<UserEarningsConfigEntry> = resp.iter()
                .map(|r| {
                    let tokens: Vec<TokenInfo> = r.tokens.iter()
                        .map(|t| TokenInfo {
                            token_id: t.token_id.to_string(),
                            outcome: t.outcome.clone(),
                            price: t.price.to_string(),
                            winner: t.winner,
                        })
                        .collect();
                    UserEarningsConfigEntry {
                        condition_id: format!("{}", r.condition_id),
                        question: r.question.clone(),
                        market_slug: r.market_slug.clone(),
                        rewards_max_spread: r.rewards_max_spread.to_string(),
                        rewards_min_size: r.rewards_min_size.to_string(),
                        earning_percentage: r.earning_percentage.to_string(),
                        maker_address: format!("{}", r.maker_address),
                        tokens,
                    }
                })
                .collect();
            ctx.output.write_data(&entries, None).map_err(io_err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BalanceAllowanceInfo, BookInfo, BookLevel, CancelResult, PostOrderResult, TradeInfo,
    };

    // ── PostOrderResult serialization ────────────────────────────────────────

    #[test]
    fn post_order_result_money_fields_are_strings() {
        let dto = PostOrderResult {
            order_id: "abc123".to_string(),
            success: true,
            status: "matched".to_string(),
            making_amount: "100.5".to_string(),
            taking_amount: "65.325".to_string(),
            error: None,
        };

        let json = serde_json::to_value(&dto).expect("serialization must succeed");

        // Amount fields must be strings, not floats, to preserve precision.
        assert!(json["making_amount"].is_string(), "making_amount must be a string");
        assert!(json["taking_amount"].is_string(), "taking_amount must be a string");
        assert!(json["success"].is_boolean(), "success must be a bool");

        // Optional error field must be absent when None (skip_serializing_if).
        assert!(json.get("error").is_none() || json["error"].is_null(),
            "error must be absent or null when None");
    }

    #[test]
    fn post_order_result_error_present_when_some() {
        let dto = PostOrderResult {
            order_id: "xyz".to_string(),
            success: false,
            status: "failed".to_string(),
            making_amount: "0".to_string(),
            taking_amount: "0".to_string(),
            error: Some("insufficient balance".to_string()),
        };

        let json = serde_json::to_value(&dto).unwrap();
        assert!(json["error"].is_string(), "error must be present as string when Some");
        assert_eq!(json["error"].as_str().unwrap(), "insufficient balance");
    }

    // ── BalanceAllowanceInfo serialization ───────────────────────────────────

    #[test]
    fn balance_allowance_info_balance_is_string() {
        let dto = BalanceAllowanceInfo {
            balance: "9999.99".to_string(),
        };

        let json = serde_json::to_value(&dto).unwrap();
        assert!(json["balance"].is_string(), "balance must serialize as string, not number");
        assert_eq!(json["balance"].as_str().unwrap(), "9999.99");
    }

    // ── BookLevel and BookInfo serialization ─────────────────────────────────

    #[test]
    fn book_level_price_and_size_are_strings() {
        let level = BookLevel {
            price: "0.65".to_string(),
            size: "500.0".to_string(),
        };

        let json = serde_json::to_value(&level).unwrap();
        assert!(json["price"].is_string(), "price must be a string");
        assert!(json["size"].is_string(), "size must be a string");
    }

    #[test]
    fn book_info_serializes_correctly() {
        let dto = BookInfo {
            market: "0xabc".to_string(),
            asset_id: "0x1".to_string(),
            bids: vec![
                BookLevel { price: "0.60".to_string(), size: "100".to_string() },
            ],
            asks: vec![
                BookLevel { price: "0.65".to_string(), size: "200".to_string() },
            ],
            min_order_size: "5".to_string(),
            tick_size: "0.01".to_string(),
            neg_risk: false,
            last_trade_price: Some("0.62".to_string()),
        };

        let json = serde_json::to_value(&dto).unwrap();
        assert!(json["bids"].is_array(), "bids must be a JSON array");
        assert!(json["asks"].is_array(), "asks must be a JSON array");
        assert_eq!(json["bids"].as_array().unwrap().len(), 1);
        assert!(json["neg_risk"].is_boolean(), "neg_risk must be a bool");
        assert!(json["last_trade_price"].is_string(), "last_trade_price must be string when Some");
    }

    #[test]
    fn book_info_last_trade_price_absent_when_none() {
        let dto = BookInfo {
            market: "0xdef".to_string(),
            asset_id: "0x2".to_string(),
            bids: vec![],
            asks: vec![],
            min_order_size: "1".to_string(),
            tick_size: "0.01".to_string(),
            neg_risk: true,
            last_trade_price: None,
        };

        let json = serde_json::to_value(&dto).unwrap();
        // skip_serializing_if = "Option::is_none" means key must be absent.
        assert!(
            json.get("last_trade_price").is_none(),
            "last_trade_price must be absent from JSON when None"
        );
    }

    // ── CancelResult serialization ───────────────────────────────────────────

    #[test]
    fn cancel_result_fields_are_string_arrays() {
        let dto = CancelResult {
            canceled: vec!["order-1".to_string(), "order-2".to_string()],
            not_canceled: vec!["order-3".to_string()],
        };

        let json = serde_json::to_value(&dto).unwrap();
        assert!(json["canceled"].is_array(), "canceled must be an array");
        assert!(json["not_canceled"].is_array(), "not_canceled must be an array");
        assert_eq!(json["canceled"].as_array().unwrap().len(), 2);
        assert_eq!(json["not_canceled"].as_array().unwrap().len(), 1);
    }

    // ── TradeInfo serialization ──────────────────────────────────────────────

    #[test]
    fn trade_info_fields_are_strings() {
        let dto = TradeInfo {
            id: "trade-abc".to_string(),
            market: "0xcondition".to_string(),
            side: "buy".to_string(),
            size: "100".to_string(),
            price: "0.65".to_string(),
            status: "matched".to_string(),
            outcome: "Yes".to_string(),
            match_time: "2025-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_value(&dto).unwrap();
        assert!(json["side"].is_string(), "side must be string");
        assert!(json["size"].is_string(), "size must be string, not number");
        assert!(json["price"].is_string(), "price must be string, not number");

        // Verify side is lowercase (not Debug-formatted).
        assert_eq!(json["side"].as_str().unwrap(), "buy");
    }
}
