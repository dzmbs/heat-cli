use std::str::FromStr;

use clap::{Args, Subcommand};
use futures::StreamExt;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use polymarket_client_sdk::clob::types::{OrderStatusType, Side};
use polymarket_client_sdk::clob::ws;
use polymarket_client_sdk::clob::ws::types::response::TradeMessageStatus;
use polymarket_client_sdk::types::{B256, U256};
use serde::Serialize;

use crate::auth;

// ── Stable string mapping helpers ────────────────────────────────────────

fn map_side(side: &Side) -> &'static str {
    match side {
        Side::Buy => "buy",
        Side::Sell => "sell",
        _ => "unknown",
    }
}

fn map_order_status(status: &OrderStatusType) -> &'static str {
    match status {
        OrderStatusType::Live => "live",
        OrderStatusType::Matched => "matched",
        OrderStatusType::Canceled => "canceled",
        OrderStatusType::Delayed => "delayed",
        OrderStatusType::Unmatched => "unmatched",
        _ => "unknown",
    }
}

fn map_trade_status(status: &TradeMessageStatus) -> &'static str {
    match status {
        TradeMessageStatus::Matched => "matched",
        TradeMessageStatus::Mined => "mined",
        TradeMessageStatus::Confirmed => "confirmed",
        _ => "unknown",
    }
}

// ── WS auth helper ───────────────────────────────────────────────────────

/// Authenticate a WS client by first deriving API credentials via the CLOB client,
/// then passing them to the WS client.
async fn authenticated_ws_client(
    ctx: &Ctx,
    sig_type_flag: Option<&str>,
) -> Result<
    ws::Client<
        polymarket_client_sdk::auth::state::Authenticated<polymarket_client_sdk::auth::Normal>,
    >,
    HeatError,
> {
    let clob = auth::authenticated_clob_client(ctx, sig_type_flag).await?;
    let credentials = clob.credentials().clone();
    let address = clob.address();

    ws::Client::default()
        .authenticate(credentials, address)
        .map_err(|e| HeatError::auth("ws_auth", format!("Failed to authenticate WS client: {e}")))
}

// ── CLI types ────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct StreamArgs {
    #[command(subcommand)]
    pub command: StreamSubcommand,
}

#[derive(Subcommand)]
pub enum StreamSubcommand {
    /// Stream real-time orderbook updates
    Orderbook(MarketStreamArgs),
    /// Stream real-time price changes
    Prices(MarketStreamArgs),
    /// Stream real-time midpoint prices
    Midpoints(MarketStreamArgs),
    /// Stream your order updates (requires account)
    Orders(UserStreamArgs),
    /// Stream your trade executions (requires account)
    Trades(UserStreamArgs),
}

#[derive(Args)]
pub struct MarketStreamArgs {
    /// Token/asset ID to subscribe to
    pub token_id: String,
}

#[derive(Args)]
pub struct UserStreamArgs {
    /// Market condition ID to filter (omit for all markets)
    #[arg(long)]
    pub market: Option<String>,
    /// Polymarket signature type
    #[arg(long)]
    pub sig_type: Option<String>,
}

pub async fn run(args: StreamArgs, ctx: &Ctx) -> Result<(), HeatError> {
    reject_quiet(ctx)?;

    match args.command {
        StreamSubcommand::Orderbook(a) => orderbook(a, ctx).await,
        StreamSubcommand::Prices(a) => prices(a, ctx).await,
        StreamSubcommand::Midpoints(a) => midpoints(a, ctx).await,
        StreamSubcommand::Orders(a) => orders(a, ctx).await,
        StreamSubcommand::Trades(a) => trades(a, ctx).await,
    }
}

// ── Parsing helpers ──────────────────────────────────────────────────────

fn parse_token_id(s: &str) -> Result<U256, HeatError> {
    U256::from_str(s).map_err(|e| {
        HeatError::validation("invalid_token_id", format!("Invalid token ID: {e}"))
            .with_hint("Token IDs are large decimal numbers from Polymarket")
    })
}

fn parse_market_id(s: &str) -> Result<B256, HeatError> {
    B256::from_str(s).map_err(|e| {
        HeatError::validation(
            "invalid_market_id",
            format!("Invalid market condition ID: {e}"),
        )
        .with_hint("Market IDs are 0x-prefixed hex strings")
    })
}

fn parse_optional_market(market: &Option<String>) -> Result<Vec<B256>, HeatError> {
    match market {
        Some(m) => Ok(vec![parse_market_id(m)?]),
        None => Ok(vec![]),
    }
}

fn reject_quiet(ctx: &Ctx) -> Result<(), HeatError> {
    if ctx.output.format == OutputFormat::Quiet {
        return Err(HeatError::validation(
            "quiet_not_supported",
            "Stream commands do not support --quiet",
        )
        .with_hint("Use --json or --ndjson for machine-readable output"));
    }
    Ok(())
}

// ── Market data streams (no auth required) ──────────────────────────────

async fn orderbook(args: MarketStreamArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let asset_id = parse_token_id(&args.token_id)?;
    let client = ws::Client::default();

    ctx.output.diagnostic(&format!(
        "Subscribing to orderbook for {}...",
        args.token_id
    ));

    let stream = client.subscribe_orderbook(vec![asset_id]).map_err(|e| {
        HeatError::network(
            "ws_subscribe",
            format!("Failed to subscribe to orderbook: {e}"),
        )
    })?;
    let mut stream = Box::pin(stream);

    while let Some(result) = stream.next().await {
        match result {
            Ok(book) => {
                let dto = OrderbookDto {
                    stream: "orderbook",
                    market: book.market.to_string(),
                    asset_id: book.asset_id.to_string(),
                    timestamp: book.timestamp,
                    bids: book
                        .bids
                        .iter()
                        .map(|l| LevelDto {
                            price: l.price.to_string(),
                            size: l.size.to_string(),
                        })
                        .collect(),
                    asks: book
                        .asks
                        .iter()
                        .map(|l| LevelDto {
                            price: l.price.to_string(),
                            size: l.size.to_string(),
                        })
                        .collect(),
                };
                emit(ctx, &dto, || {
                    let best_bid = dto.bids.first().map(|l| l.price.as_str()).unwrap_or("-");
                    let best_ask = dto.asks.first().map(|l| l.price.as_str()).unwrap_or("-");
                    format!(
                        "book {} bid {} x {} ask {} x {}",
                        short_id(&dto.asset_id),
                        best_bid,
                        dto.bids.first().map(|l| l.size.as_str()).unwrap_or("0"),
                        best_ask,
                        dto.asks.first().map(|l| l.size.as_str()).unwrap_or("0"),
                    )
                })?;
            }
            Err(e) => {
                ctx.output.diagnostic(&format!("Stream error: {e}"));
            }
        }
    }

    Ok(())
}

async fn prices(args: MarketStreamArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let asset_id = parse_token_id(&args.token_id)?;
    let client = ws::Client::default();

    ctx.output
        .diagnostic(&format!("Subscribing to prices for {}...", args.token_id));

    let stream = client.subscribe_prices(vec![asset_id]).map_err(|e| {
        HeatError::network(
            "ws_subscribe",
            format!("Failed to subscribe to prices: {e}"),
        )
    })?;
    let mut stream = Box::pin(stream);

    while let Some(result) = stream.next().await {
        match result {
            Ok(change) => {
                let dto = PriceChangeDto {
                    stream: "prices",
                    market: change.market.to_string(),
                    timestamp: change.timestamp,
                    changes: change
                        .price_changes
                        .iter()
                        .map(|pc| PriceEntryDto {
                            asset_id: pc.asset_id.to_string(),
                            price: pc.price.to_string(),
                            size: pc.size.map(|s| s.to_string()),
                            side: map_side(&pc.side).to_string(),
                            best_bid: pc.best_bid.map(|b| b.to_string()),
                            best_ask: pc.best_ask.map(|a| a.to_string()),
                        })
                        .collect(),
                };
                match ctx.output.format {
                    OutputFormat::Pretty => {
                        for c in &dto.changes {
                            println!(
                                "price {} {} {}{}",
                                short_id(&c.asset_id),
                                c.side,
                                c.price,
                                c.size
                                    .as_ref()
                                    .map(|s| format!(" size {s}"))
                                    .unwrap_or_default(),
                            );
                        }
                    }
                    OutputFormat::Json | OutputFormat::Ndjson => {
                        ctx.output.write_ndjson(&dto).map_err(|e| {
                            HeatError::internal(
                                "write_failed",
                                format!("Failed to write stream event: {e}"),
                            )
                        })?;
                    }
                    OutputFormat::Quiet => {}
                }
            }
            Err(e) => {
                ctx.output.diagnostic(&format!("Stream error: {e}"));
            }
        }
    }

    Ok(())
}

async fn midpoints(args: MarketStreamArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let asset_id = parse_token_id(&args.token_id)?;
    let client = ws::Client::default();

    ctx.output.diagnostic(&format!(
        "Subscribing to midpoints for {}...",
        args.token_id
    ));

    let stream = client.subscribe_midpoints(vec![asset_id]).map_err(|e| {
        HeatError::network(
            "ws_subscribe",
            format!("Failed to subscribe to midpoints: {e}"),
        )
    })?;
    let mut stream = Box::pin(stream);

    while let Some(result) = stream.next().await {
        match result {
            Ok(mid) => {
                let dto = MidpointDto {
                    stream: "midpoints",
                    market: mid.market.to_string(),
                    asset_id: mid.asset_id.to_string(),
                    timestamp: mid.timestamp,
                    midpoint: mid.midpoint.to_string(),
                };
                emit(ctx, &dto, || {
                    format!("midpoint {} {}", short_id(&dto.asset_id), dto.midpoint)
                })?;
            }
            Err(e) => {
                ctx.output.diagnostic(&format!("Stream error: {e}"));
            }
        }
    }

    Ok(())
}

// ── Authenticated user streams ──────────────────────────────────────────

async fn orders(args: UserStreamArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let markets = parse_optional_market(&args.market)?;

    ctx.output.diagnostic("Authenticating...");

    let client = authenticated_ws_client(ctx, args.sig_type.as_deref()).await?;

    if markets.is_empty() {
        ctx.output
            .diagnostic("Subscribing to order updates (all markets)...");
    } else {
        ctx.output.diagnostic("Subscribing to order updates...");
    }

    let stream = client.subscribe_orders(markets).map_err(|e| {
        HeatError::network(
            "ws_subscribe",
            format!("Failed to subscribe to orders: {e}"),
        )
    })?;
    let mut stream = Box::pin(stream);

    while let Some(result) = stream.next().await {
        match result {
            Ok(order) => {
                let dto = OrderDto {
                    stream: "orders",
                    id: order.id.clone(),
                    market: order.market.to_string(),
                    asset_id: order.asset_id.to_string(),
                    side: map_side(&order.side).to_string(),
                    price: order.price.to_string(),
                    status: order
                        .status
                        .as_ref()
                        .map(|s| map_order_status(s).to_string()),
                    outcome: order.outcome.clone(),
                    original_size: order.original_size.map(|s| s.to_string()),
                    size_matched: order.size_matched.map(|s| s.to_string()),
                    timestamp: order.timestamp,
                };
                emit(ctx, &dto, || {
                    format!(
                        "order {} {} {} @ {}{}",
                        short_id(&dto.id),
                        dto.side,
                        dto.original_size.as_deref().unwrap_or("-"),
                        dto.price,
                        dto.status
                            .as_ref()
                            .map(|s| format!(" {s}"))
                            .unwrap_or_default(),
                    )
                })?;
            }
            Err(e) => {
                ctx.output.diagnostic(&format!("Stream error: {e}"));
            }
        }
    }

    Ok(())
}

async fn trades(args: UserStreamArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let markets = parse_optional_market(&args.market)?;

    ctx.output.diagnostic("Authenticating...");

    let client = authenticated_ws_client(ctx, args.sig_type.as_deref()).await?;

    if markets.is_empty() {
        ctx.output
            .diagnostic("Subscribing to trade updates (all markets)...");
    } else {
        ctx.output.diagnostic("Subscribing to trade updates...");
    }

    let stream = client.subscribe_trades(markets).map_err(|e| {
        HeatError::network(
            "ws_subscribe",
            format!("Failed to subscribe to trades: {e}"),
        )
    })?;
    let mut stream = Box::pin(stream);

    while let Some(result) = stream.next().await {
        match result {
            Ok(trade) => {
                let dto = TradeDto {
                    stream: "trades",
                    id: trade.id.clone(),
                    market: trade.market.to_string(),
                    asset_id: trade.asset_id.to_string(),
                    side: map_side(&trade.side).to_string(),
                    size: trade.size.to_string(),
                    price: trade.price.to_string(),
                    status: map_trade_status(&trade.status).to_string(),
                    outcome: trade.outcome.clone(),
                    match_time: trade.matchtime,
                    timestamp: trade.timestamp,
                };
                emit(ctx, &dto, || {
                    format!(
                        "trade {} {} {} @ {} {}",
                        short_id(&dto.id),
                        dto.side,
                        dto.size,
                        dto.price,
                        dto.outcome.as_deref().unwrap_or(""),
                    )
                })?;
            }
            Err(e) => {
                ctx.output.diagnostic(&format!("Stream error: {e}"));
            }
        }
    }

    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn short_id(id: &str) -> &str {
    if id.len() > 12 { &id[..12] } else { id }
}

fn emit<T: Serialize, F: FnOnce() -> String>(
    ctx: &Ctx,
    dto: &T,
    pretty_fn: F,
) -> Result<(), HeatError> {
    match ctx.output.format {
        OutputFormat::Pretty => {
            println!("{}", pretty_fn());
        }
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_ndjson(dto).map_err(|e| {
                HeatError::internal("write_failed", format!("Failed to write stream event: {e}"))
            })?;
        }
        OutputFormat::Quiet => {
            // Rejected at run() entry; unreachable in normal operation.
        }
    }
    Ok(())
}

// ── DTOs ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OrderbookDto<'a> {
    stream: &'a str,
    market: String,
    asset_id: String,
    timestamp: i64,
    bids: Vec<LevelDto>,
    asks: Vec<LevelDto>,
}

#[derive(Serialize)]
struct LevelDto {
    price: String,
    size: String,
}

#[derive(Serialize)]
struct PriceChangeDto<'a> {
    stream: &'a str,
    market: String,
    timestamp: i64,
    changes: Vec<PriceEntryDto>,
}

#[derive(Serialize)]
struct PriceEntryDto {
    asset_id: String,
    price: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<String>,
    side: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    best_bid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    best_ask: Option<String>,
}

#[derive(Serialize)]
struct MidpointDto<'a> {
    stream: &'a str,
    market: String,
    asset_id: String,
    timestamp: i64,
    midpoint: String,
}

#[derive(Serialize)]
struct OrderDto<'a> {
    stream: &'a str,
    id: String,
    market: String,
    asset_id: String,
    side: String,
    price: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_matched: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<i64>,
}

#[derive(Serialize)]
struct TradeDto<'a> {
    stream: &'a str,
    id: String,
    market: String,
    asset_id: String,
    side: String,
    size: String,
    price: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    match_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<i64>,
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_token_id ───────────────────────────────────────────────

    #[test]
    fn parse_token_id_valid() {
        let result = parse_token_id(
            "15871154585880608648532107628464183779895785213830018178010423617714102767076",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn parse_token_id_zero() {
        assert!(parse_token_id("0").is_ok());
    }

    #[test]
    fn parse_token_id_accepts_hex() {
        // U256 supports 0x-prefixed hex
        assert!(parse_token_id("0xabc").is_ok());
    }

    #[test]
    fn parse_token_id_rejects_garbage() {
        assert!(parse_token_id("not_a_number").is_err());
    }

    #[test]
    fn parse_token_id_rejects_negative() {
        assert!(parse_token_id("-1").is_err());
    }

    #[test]
    fn parse_token_id_error_has_hint() {
        let err = parse_token_id("not_a_number").unwrap_err();
        let debug = format!("{err:?}");
        assert!(debug.contains("Token IDs") || format!("{err}").contains("Token ID"));
    }

    // ── parse_market_id ──────────────────────────────────────────────

    #[test]
    fn parse_market_id_valid() {
        let hex = "0x0000000000000000000000000000000000000000000000000000000000000001";
        assert!(parse_market_id(hex).is_ok());
    }

    #[test]
    fn parse_market_id_rejects_short_hex() {
        assert!(parse_market_id("0xabc").is_err());
    }

    #[test]
    fn parse_market_id_rejects_decimal() {
        assert!(parse_market_id("12345").is_err());
    }

    #[test]
    fn parse_market_id_rejects_empty() {
        assert!(parse_market_id("").is_err());
    }

    #[test]
    fn parse_market_id_error_has_hint() {
        let err = parse_market_id("bad").unwrap_err();
        let debug = format!("{err:?}");
        assert!(debug.contains("0x-prefixed") || format!("{err}").contains("Market"));
    }

    // ── map_side ─────────────────────────────────────────────────────

    #[test]
    fn map_side_buy() {
        assert_eq!(map_side(&Side::Buy), "buy");
    }

    #[test]
    fn map_side_sell() {
        assert_eq!(map_side(&Side::Sell), "sell");
    }

    #[test]
    fn map_side_unknown() {
        assert_eq!(map_side(&Side::Unknown), "unknown");
    }

    // ── map_order_status ─────────────────────────────────────────────

    #[test]
    fn map_order_status_all_known() {
        assert_eq!(map_order_status(&OrderStatusType::Live), "live");
        assert_eq!(map_order_status(&OrderStatusType::Matched), "matched");
        assert_eq!(map_order_status(&OrderStatusType::Canceled), "canceled");
        assert_eq!(map_order_status(&OrderStatusType::Delayed), "delayed");
        assert_eq!(map_order_status(&OrderStatusType::Unmatched), "unmatched");
    }

    #[test]
    fn map_order_status_unknown_variant() {
        assert_eq!(
            map_order_status(&OrderStatusType::Unknown("NEW_STATUS".to_string())),
            "unknown"
        );
    }

    // ── map_trade_status ─────────────────────────────────────────────

    #[test]
    fn map_trade_status_all_known() {
        assert_eq!(map_trade_status(&TradeMessageStatus::Matched), "matched");
        assert_eq!(map_trade_status(&TradeMessageStatus::Mined), "mined");
        assert_eq!(
            map_trade_status(&TradeMessageStatus::Confirmed),
            "confirmed"
        );
    }

    #[test]
    fn map_trade_status_unknown_variant() {
        assert_eq!(
            map_trade_status(&TradeMessageStatus::Unknown("PENDING".to_string())),
            "unknown"
        );
    }

    // ── short_id ─────────────────────────────────────────────────────

    #[test]
    fn short_id_truncates_long() {
        assert_eq!(short_id("abcdefghijklmnop"), "abcdefghijkl");
    }

    #[test]
    fn short_id_keeps_short() {
        assert_eq!(short_id("abc"), "abc");
    }

    #[test]
    fn short_id_exact_boundary() {
        assert_eq!(short_id("abcdefghijkl"), "abcdefghijkl");
    }

    #[test]
    fn short_id_empty() {
        assert_eq!(short_id(""), "");
    }

    // ── DTO serialization stability ──────────────────────────────────

    #[test]
    fn orderbook_dto_shape() {
        let dto = OrderbookDto {
            stream: "orderbook",
            market: "0xabc".to_string(),
            asset_id: "123".to_string(),
            timestamp: 1700000000,
            bids: vec![LevelDto {
                price: "0.55".to_string(),
                size: "100".to_string(),
            }],
            asks: vec![],
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&dto).unwrap()).unwrap();
        assert_eq!(json["stream"], "orderbook");
        assert_eq!(json["market"], "0xabc");
        assert_eq!(json["timestamp"], 1700000000);
        assert_eq!(json["bids"][0]["price"], "0.55");
        assert_eq!(json["bids"][0]["size"], "100");
        assert!(json["asks"].as_array().unwrap().is_empty());
    }

    #[test]
    fn midpoint_dto_shape() {
        let dto = MidpointDto {
            stream: "midpoints",
            market: "0xabc".to_string(),
            asset_id: "123".to_string(),
            timestamp: 1700000000,
            midpoint: "0.52".to_string(),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&dto).unwrap()).unwrap();
        assert_eq!(json["stream"], "midpoints");
        assert_eq!(json["midpoint"], "0.52");
    }

    #[test]
    fn order_dto_skips_none_fields() {
        let dto = OrderDto {
            stream: "orders",
            id: "abc".to_string(),
            market: "0x1".to_string(),
            asset_id: "999".to_string(),
            side: "buy".to_string(),
            price: "0.60".to_string(),
            status: None,
            outcome: None,
            original_size: None,
            size_matched: None,
            timestamp: None,
        };
        let json_str = serde_json::to_string(&dto).unwrap();
        assert!(!json_str.contains("status"));
        assert!(!json_str.contains("outcome"));
        assert!(!json_str.contains("original_size"));
    }

    #[test]
    fn trade_dto_shape() {
        let dto = TradeDto {
            stream: "trades",
            id: "t1".to_string(),
            market: "0xabc".to_string(),
            asset_id: "123".to_string(),
            side: "sell".to_string(),
            size: "50".to_string(),
            price: "0.70".to_string(),
            status: "confirmed".to_string(),
            outcome: Some("Yes".to_string()),
            match_time: Some(1700000000),
            timestamp: Some(1700000001),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&dto).unwrap()).unwrap();
        assert_eq!(json["stream"], "trades");
        assert_eq!(json["side"], "sell");
        assert_eq!(json["status"], "confirmed");
        assert_eq!(json["outcome"], "Yes");
        assert_eq!(json["match_time"], 1700000000);
    }

    #[test]
    fn price_entry_dto_skips_none_fields() {
        let dto = PriceEntryDto {
            asset_id: "123".to_string(),
            price: "0.50".to_string(),
            size: None,
            side: "buy".to_string(),
            best_bid: None,
            best_ask: None,
        };
        let json_str = serde_json::to_string(&dto).unwrap();
        assert!(!json_str.contains("size"));
        assert!(!json_str.contains("best_bid"));
        assert!(!json_str.contains("best_ask"));
        assert!(json_str.contains("\"side\":\"buy\""));
    }
}
