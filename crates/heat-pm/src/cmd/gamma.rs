//! Gamma API commands — markets, events, tags, series, comments, profiles, sports.

use clap::Subcommand;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use polymarket_client_sdk::gamma;
use polymarket_client_sdk::gamma::types::ParentEntityType;
use polymarket_client_sdk::gamma::types::request::{
    CommentsByIdRequest, CommentsByUserAddressRequest, CommentsRequest, EventByIdRequest,
    EventBySlugRequest, EventTagsRequest, EventsRequest, MarketByIdRequest, MarketBySlugRequest,
    MarketTagsRequest, MarketsRequest, PublicProfileRequest, RelatedTagsByIdRequest,
    RelatedTagsBySlugRequest, SearchRequest, SeriesByIdRequest, SeriesListRequest, TagByIdRequest,
    TagBySlugRequest, TagsRequest, TeamsRequest,
};
use polymarket_client_sdk::types::Address;
use serde::Serialize;
use std::collections::HashSet;

fn gamma_client() -> gamma::Client {
    gamma::Client::default()
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}

fn gamma_err(e: impl std::fmt::Display) -> HeatError {
    HeatError::network("gamma_request", format!("Gamma API error: {e}"))
}

fn parse_address(s: &str) -> Result<Address, HeatError> {
    s.parse::<Address>()
        .map_err(|e| HeatError::internal("address_parse", format!("Invalid address '{s}': {e}")))
}

// ── Heat-owned Market DTO ──────────────────────────────────────────────────

/// Normalized market representation. All money fields are strings.
#[derive(Debug, Serialize)]
struct MarketDto {
    condition_id: String,
    question: String,
    slug: String,
    active: bool,
    closed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    yes_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_price: Option<String>,
    volume: String,
    liquidity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_date: Option<String>,
}

impl MarketDto {
    fn from_sdk(m: &polymarket_client_sdk::gamma::types::response::Market) -> Self {
        let prices = m.outcome_prices.as_ref();
        let yes_price = prices.and_then(|p| p.first()).map(|d| d.to_string());
        let no_price = prices.and_then(|p| p.get(1)).map(|d| d.to_string());
        Self {
            condition_id: m.condition_id.map(|c| format!("{c}")).unwrap_or_default(),
            question: m.question.clone().unwrap_or_default(),
            slug: m.slug.clone().unwrap_or_default(),
            active: m.active.unwrap_or(false),
            closed: m.closed.unwrap_or(false),
            yes_price,
            no_price,
            volume: m
                .volume
                .map(|v| v.to_string())
                .unwrap_or_else(|| "0".to_string()),
            liquidity: m
                .liquidity
                .map(|l| l.to_string())
                .unwrap_or_else(|| "0".to_string()),
            end_date: m.end_date.map(|d| d.to_rfc3339()),
        }
    }

    fn pretty_row(&self) -> String {
        let yes = self.yes_price.as_deref().unwrap_or("-");
        let no = self.no_price.as_deref().unwrap_or("-");
        let status = if self.closed {
            "closed"
        } else if self.active {
            "active"
        } else {
            "inactive"
        };
        format!(
            "  {:<8} yes={:<6} no={:<6} vol={:<12} liq={:<12} {}",
            status, yes, no, self.volume, self.liquidity, self.question
        )
    }
}

fn dedupe_markets(markets: Vec<MarketDto>, key: &str) -> Vec<MarketDto> {
    let mut seen = HashSet::new();
    markets
        .into_iter()
        .filter(|m| {
            let k = match key {
                "slug" => &m.slug,
                _ => &m.condition_id,
            };
            seen.insert(k.clone())
        })
        .collect()
}

fn sort_markets(markets: &mut [MarketDto], sort: &str) {
    match sort {
        "volume" => markets.sort_by(|a, b| {
            let va: f64 = a.volume.parse().unwrap_or(0.0);
            let vb: f64 = b.volume.parse().unwrap_or(0.0);
            vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
        }),
        "liquidity" => markets.sort_by(|a, b| {
            let la: f64 = a.liquidity.parse().unwrap_or(0.0);
            let lb: f64 = b.liquidity.parse().unwrap_or(0.0);
            lb.partial_cmp(&la).unwrap_or(std::cmp::Ordering::Equal)
        }),
        "newest" => markets.sort_by(|a, b| {
            let da = a.end_date.as_deref().unwrap_or("");
            let db = b.end_date.as_deref().unwrap_or("");
            db.cmp(da)
        }),
        _ => {}
    }
}

fn output_markets(markets: &[MarketDto], ctx: &Ctx) -> Result<(), HeatError> {
    match ctx.output.format {
        OutputFormat::Pretty => {
            if markets.is_empty() {
                println!("  No markets found.");
            } else {
                for m in markets {
                    println!("{}", m.pretty_row());
                }
            }
        }
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&markets, None).map_err(io_err)?;
        }
        OutputFormat::Quiet => {}
    }
    Ok(())
}

// ── Markets ──────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum MarketsSubcommand {
    /// List markets
    List {
        /// Maximum number of results
        #[arg(long, default_value_t = 25)]
        limit: i32,
        /// Offset for pagination
        #[arg(long)]
        offset: Option<i32>,
        /// Filter by closed status
        #[arg(long)]
        closed: Option<bool>,
        /// Filter by active status (client-side)
        #[arg(long)]
        active: Option<bool>,
        /// Sort results: volume, liquidity, newest
        #[arg(long)]
        sort: Option<String>,
        /// Deduplicate by: slug, condition_id
        #[arg(long)]
        dedupe: Option<String>,
    },
    /// Get market by ID or slug
    Get {
        /// Market ID or slug
        id: String,
    },
    /// Search markets by query (extracts markets from matching events)
    Search {
        /// Search query
        query: String,
        /// Max results per type
        #[arg(long, default_value_t = 10)]
        limit: i32,
        /// Filter by closed status (client-side)
        #[arg(long)]
        closed: Option<bool>,
        /// Filter by active status (client-side)
        #[arg(long)]
        active: Option<bool>,
        /// Sort results: volume, liquidity, newest
        #[arg(long)]
        sort: Option<String>,
        /// Deduplicate by: slug, condition_id
        #[arg(long)]
        dedupe: Option<String>,
    },
    /// Get tags for a market
    Tags {
        /// Market ID
        id: String,
    },
}

pub async fn markets(sub: MarketsSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    let client = gamma_client();
    match sub {
        MarketsSubcommand::List {
            limit,
            offset,
            closed,
            active,
            sort,
            dedupe,
        } => {
            let req = MarketsRequest::builder()
                .limit(limit)
                .maybe_offset(offset)
                .maybe_closed(closed)
                .build();
            let raw = client.markets(&req).await.map_err(gamma_err)?;
            let mut dtos: Vec<MarketDto> = raw.iter().map(MarketDto::from_sdk).collect();

            // Client-side active filter (SDK doesn't support it for markets)
            if let Some(want_active) = active {
                dtos.retain(|m| m.active == want_active);
            }
            if let Some(ref key) = dedupe {
                dtos = dedupe_markets(dtos, key);
            }
            if let Some(ref s) = sort {
                sort_markets(&mut dtos, s);
            }

            output_markets(&dtos, ctx)
        }
        MarketsSubcommand::Get { id } => {
            let result = if id.starts_with("0x") || id.len() == 66 {
                let req = MarketByIdRequest::builder().id(&id).build();
                client.market_by_id(&req).await.map_err(gamma_err)?
            } else {
                let req = MarketBySlugRequest::builder().slug(&id).build();
                client.market_by_slug(&req).await.map_err(gamma_err)?
            };
            let dto = MarketDto::from_sdk(&result);
            match ctx.output.format {
                OutputFormat::Pretty => {
                    println!("Question:     {}", dto.question);
                    println!("Condition ID: {}", dto.condition_id);
                    println!("Slug:         {}", dto.slug);
                    println!("Active:       {}", dto.active);
                    println!("Closed:       {}", dto.closed);
                    if let Some(ref y) = dto.yes_price {
                        println!("Yes price:    {y}");
                    }
                    if let Some(ref n) = dto.no_price {
                        println!("No price:     {n}");
                    }
                    println!("Volume:       {}", dto.volume);
                    println!("Liquidity:    {}", dto.liquidity);
                    if let Some(ref d) = dto.end_date {
                        println!("End date:     {d}");
                    }
                }
                OutputFormat::Json | OutputFormat::Ndjson => {
                    ctx.output.write_data(&dto, None).map_err(io_err)?;
                }
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        MarketsSubcommand::Search {
            query,
            limit,
            closed,
            active,
            sort,
            dedupe,
        } => {
            let req = SearchRequest::builder()
                .q(&query)
                .limit_per_type(limit)
                .build();
            let results = client.search(&req).await.map_err(gamma_err)?;

            // Extract markets from events (search returns events, not markets directly)
            let mut dtos = Vec::new();
            if let Some(events) = &results.events {
                for event in events {
                    if let Some(markets) = &event.markets {
                        for m in markets {
                            dtos.push(MarketDto::from_sdk(m));
                        }
                    }
                }
            }

            // Apply client-side filters
            if let Some(want_closed) = closed {
                dtos.retain(|m| m.closed == want_closed);
            }
            if let Some(want_active) = active {
                dtos.retain(|m| m.active == want_active);
            }
            if let Some(ref key) = dedupe {
                dtos = dedupe_markets(dtos, key);
            }
            if let Some(ref s) = sort {
                sort_markets(&mut dtos, s);
            }

            output_markets(&dtos, ctx)
        }
        MarketsSubcommand::Tags { id } => {
            let req = MarketTagsRequest::builder().id(&id).build();
            let tags = client.market_tags(&req).await.map_err(gamma_err)?;
            ctx.output.write_data(&tags, None).map_err(io_err)
        }
    }
}

// ── Events ───────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum EventsSubcommand {
    /// List events
    List {
        #[arg(long, default_value_t = 25)]
        limit: i32,
        #[arg(long)]
        offset: Option<i32>,
        #[arg(long)]
        active: Option<bool>,
        #[arg(long)]
        closed: Option<bool>,
    },
    /// Get event by ID or slug
    Get { id: String },
    /// Get tags for an event
    Tags { id: String },
}

pub async fn events(sub: EventsSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    let client = gamma_client();
    match sub {
        EventsSubcommand::List {
            limit,
            offset,
            active,
            closed,
        } => {
            let req = EventsRequest::builder()
                .limit(limit)
                .maybe_offset(offset)
                .maybe_active(active)
                .maybe_closed(closed)
                .build();
            let events = client.events(&req).await.map_err(gamma_err)?;
            match ctx.output.format {
                OutputFormat::Pretty => {
                    for e in &events {
                        println!("{:<12} {}", e.id, e.title.as_deref().unwrap_or(""));
                    }
                }
                OutputFormat::Json | OutputFormat::Ndjson => {
                    ctx.output.write_data(&events, None).map_err(io_err)?;
                }
                OutputFormat::Quiet => {}
            }
            Ok(())
        }
        EventsSubcommand::Get { id } => {
            let result = if id.parse::<i64>().is_ok() {
                let req = EventByIdRequest::builder().id(&id).build();
                client.event_by_id(&req).await.map_err(gamma_err)?
            } else {
                let req = EventBySlugRequest::builder().slug(&id).build();
                client.event_by_slug(&req).await.map_err(gamma_err)?
            };
            ctx.output.write_data(&result, None).map_err(io_err)
        }
        EventsSubcommand::Tags { id } => {
            let req = EventTagsRequest::builder().id(&id).build();
            let tags = client.event_tags(&req).await.map_err(gamma_err)?;
            ctx.output.write_data(&tags, None).map_err(io_err)
        }
    }
}

// ── Tags ─────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum TagsSubcommand {
    /// List all tags
    List {
        #[arg(long, default_value_t = 50)]
        limit: i32,
        #[arg(long)]
        offset: Option<i32>,
    },
    /// Get tag by ID or slug
    Get { id: String },
    /// Get related tags
    Related {
        /// Tag ID or slug
        id: String,
        /// Omit tags with no markets
        #[arg(long)]
        omit_empty: bool,
    },
    /// Get tags related to another tag
    RelatedToTag {
        /// Tag ID or slug
        id: String,
        /// Omit tags with no markets
        #[arg(long)]
        omit_empty: bool,
    },
}

pub async fn tags(sub: TagsSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    let client = gamma_client();
    match sub {
        TagsSubcommand::List { limit, offset } => {
            let req = TagsRequest::builder()
                .limit(limit)
                .maybe_offset(offset)
                .build();
            let tags = client.tags(&req).await.map_err(gamma_err)?;
            ctx.output.write_data(&tags, None).map_err(io_err)
        }
        TagsSubcommand::Get { id } => {
            let result = if id.parse::<i64>().is_ok() {
                let req = TagByIdRequest::builder().id(&id).build();
                client.tag_by_id(&req).await.map_err(gamma_err)?
            } else {
                let req = TagBySlugRequest::builder().slug(&id).build();
                client.tag_by_slug(&req).await.map_err(gamma_err)?
            };
            ctx.output.write_data(&result, None).map_err(io_err)
        }
        TagsSubcommand::Related { id, omit_empty } => {
            let result = if id.parse::<i64>().is_ok() {
                let req = RelatedTagsByIdRequest::builder()
                    .id(&id)
                    .maybe_omit_empty(if omit_empty { Some(true) } else { None })
                    .build();
                client.related_tags_by_id(&req).await.map_err(gamma_err)?
            } else {
                let req = RelatedTagsBySlugRequest::builder()
                    .slug(&id)
                    .maybe_omit_empty(if omit_empty { Some(true) } else { None })
                    .build();
                client.related_tags_by_slug(&req).await.map_err(gamma_err)?
            };
            ctx.output.write_data(&result, None).map_err(io_err)
        }
        TagsSubcommand::RelatedToTag { id, omit_empty } => {
            let result = if id.parse::<i64>().is_ok() {
                let req = RelatedTagsByIdRequest::builder()
                    .id(&id)
                    .maybe_omit_empty(if omit_empty { Some(true) } else { None })
                    .build();
                client
                    .tags_related_to_tag_by_id(&req)
                    .await
                    .map_err(gamma_err)?
            } else {
                let req = RelatedTagsBySlugRequest::builder()
                    .slug(&id)
                    .maybe_omit_empty(if omit_empty { Some(true) } else { None })
                    .build();
                client
                    .tags_related_to_tag_by_slug(&req)
                    .await
                    .map_err(gamma_err)?
            };
            ctx.output.write_data(&result, None).map_err(io_err)
        }
    }
}

// ── Series ───────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum SeriesSubcommand {
    /// List series
    List,
    /// Get series by ID
    Get { id: String },
}

pub async fn series(sub: SeriesSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    let client = gamma_client();
    match sub {
        SeriesSubcommand::List => {
            let req = SeriesListRequest::builder().build();
            let series = client.series(&req).await.map_err(gamma_err)?;
            ctx.output.write_data(&series, None).map_err(io_err)
        }
        SeriesSubcommand::Get { id } => {
            let req = SeriesByIdRequest::builder().id(&id).build();
            let s = client.series_by_id(&req).await.map_err(gamma_err)?;
            ctx.output.write_data(&s, None).map_err(io_err)
        }
    }
}

// ── Comments ─────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum CommentsSubcommand {
    /// List comments for an entity
    List {
        /// Parent entity type (e.g., "market", "event", "series")
        #[arg(long)]
        entity_type: String,
        /// Parent entity ID
        #[arg(long)]
        entity_id: String,
    },
    /// Get comment by ID
    Get { id: String },
    /// Get comments by user address
    ByUser { address: String },
}

pub async fn comments(sub: CommentsSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    let client = gamma_client();
    match sub {
        CommentsSubcommand::List {
            entity_type,
            entity_id,
        } => {
            let parent_type = match entity_type.to_lowercase().as_str() {
                "event" => ParentEntityType::Event,
                "series" => ParentEntityType::Series,
                "market" => ParentEntityType::Market,
                other => ParentEntityType::Unknown(other.to_owned()),
            };
            let req = CommentsRequest::builder()
                .parent_entity_type(parent_type)
                .parent_entity_id(&entity_id)
                .build();
            let comments = client.comments(&req).await.map_err(gamma_err)?;
            ctx.output.write_data(&comments, None).map_err(io_err)
        }
        CommentsSubcommand::Get { id } => {
            let req = CommentsByIdRequest::builder().id(&id).build();
            let comment = client.comments_by_id(&req).await.map_err(gamma_err)?;
            ctx.output.write_data(&comment, None).map_err(io_err)
        }
        CommentsSubcommand::ByUser { address } => {
            let addr = parse_address(&address)?;
            let req = CommentsByUserAddressRequest::builder()
                .user_address(addr)
                .build();
            let comments = client
                .comments_by_user_address(&req)
                .await
                .map_err(gamma_err)?;
            ctx.output.write_data(&comments, None).map_err(io_err)
        }
    }
}

// ── Profiles ─────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ProfilesSubcommand {
    /// Get public profile by address
    Get {
        /// Ethereum address
        address: String,
    },
}

pub async fn profiles(sub: ProfilesSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    let client = gamma_client();
    match sub {
        ProfilesSubcommand::Get { address } => {
            let addr = parse_address(&address)?;
            let req = PublicProfileRequest::builder().address(addr).build();
            let profile = client.public_profile(&req).await.map_err(gamma_err)?;
            ctx.output.write_data(&profile, None).map_err(io_err)
        }
    }
}

// ── Sports ───────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum SportsSubcommand {
    /// List sports
    List,
    /// List sports market types
    MarketTypes,
    /// List teams
    Teams,
}

pub async fn sports(sub: SportsSubcommand, ctx: &Ctx) -> Result<(), HeatError> {
    let client = gamma_client();
    match sub {
        SportsSubcommand::List => {
            let sports = client.sports().await.map_err(gamma_err)?;
            ctx.output.write_data(&sports, None).map_err(io_err)
        }
        SportsSubcommand::MarketTypes => {
            let types = client.sports_market_types().await.map_err(gamma_err)?;
            ctx.output.write_data(&types, None).map_err(io_err)
        }
        SportsSubcommand::Teams => {
            let req = TeamsRequest::builder().build();
            let teams = client.teams(&req).await.map_err(gamma_err)?;
            ctx.output.write_data(&teams, None).map_err(io_err)
        }
    }
}

// ── Status ───────────────────────────────────────────────────────────────

pub async fn status(ctx: &Ctx) -> Result<(), HeatError> {
    let client = gamma_client();
    let status = client.status().await.map_err(gamma_err)?;
    match ctx.output.format {
        OutputFormat::Pretty => println!("Gamma API: {status}"),
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output
                .write_data(&serde_json::json!({ "status": status }), None)
                .map_err(io_err)?;
        }
        OutputFormat::Quiet => {}
    }
    Ok(())
}
