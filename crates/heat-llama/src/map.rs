/// Mapping from raw DefiLlama API responses to Heat-owned DTOs.
use crate::client;
use crate::dto;

// ---------------------------------------------------------------------------
// Protocols
// ---------------------------------------------------------------------------

pub fn map_protocols(raw: Vec<client::RawProtocol>) -> dto::ProtocolsListDto {
    let protocols = raw
        .into_iter()
        .map(|p| dto::ProtocolRow {
            id: p.id.unwrap_or_default(),
            slug: p.slug.unwrap_or_default(),
            name: p.name.unwrap_or_default(),
            symbol: p.symbol,
            category: p.category,
            chains: p.chains,
            tvl_usd: p.tvl,
            change_1d_pct: p.change_1d,
            change_7d_pct: p.change_7d,
            change_1m_pct: p.change_1m,
            url: p.url,
        })
        .collect();
    dto::ProtocolsListDto { protocols }
}

pub fn map_protocol_detail(
    raw: client::RawProtocolDetail,
    requested_slug: &str,
) -> dto::ProtocolDetailDto {
    // TVL can be a number or an array of historical points. Extract current TVL.
    let tvl_usd = match &raw.tvl {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::Array(arr)) => arr
            .last()
            .and_then(|v| v.get("totalLiquidityUSD"))
            .and_then(|v| v.as_f64()),
        _ => None,
    };

    // Preserve requested slug — upstream may not include it.
    let slug = raw
        .slug
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| requested_slug.to_owned());

    dto::ProtocolDetailDto {
        id: raw.id.unwrap_or_default(),
        slug,
        name: raw.name.unwrap_or_default(),
        symbol: raw.symbol,
        category: raw.category,
        description: raw.description,
        url: raw.url,
        chains: raw.chains,
        tvl_usd,
        chain_tvls: raw.current_chain_tvls,
        mcap_usd: raw.mcap,
    }
}

// ---------------------------------------------------------------------------
// Chains
// ---------------------------------------------------------------------------

pub fn map_chains(raw: Vec<client::RawChain>) -> dto::ChainsListDto {
    let chains = raw
        .into_iter()
        .map(|c| dto::ChainRow {
            name: c.name.unwrap_or_default(),
            token_symbol: c.token_symbol,
            tvl_usd: c.tvl,
            chain_id: parse_chain_id(&c.chain_id),
            gecko_id: c.gecko_id,
        })
        .collect();
    dto::ChainsListDto { chains }
}

/// Parse chain_id which can be a number or string in upstream data.
fn parse_chain_id(val: &Option<serde_json::Value>) -> Option<u64> {
    match val {
        Some(serde_json::Value::Number(n)) => n.as_u64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

pub fn map_chain_history(
    raw: Vec<client::RawTvlPoint>,
    chain: Option<&str>,
) -> dto::ChainHistoryDto {
    let points = raw
        .into_iter()
        .filter_map(|p| {
            let date = p.date?;
            let tvl = p.tvl.or(p.total_liquidity_usd)?;
            Some(dto::TvlPoint { date, tvl_usd: tvl })
        })
        .collect();
    dto::ChainHistoryDto {
        chain: chain.map(|s| s.to_owned()),
        points,
    }
}

// ---------------------------------------------------------------------------
// Coins
// ---------------------------------------------------------------------------

pub fn map_coins_price(raw: client::RawCoinsResponse) -> dto::CoinsPriceDto {
    let mut prices: Vec<dto::CoinPrice> = raw
        .coins
        .into_iter()
        .map(|(key, val)| dto::CoinPrice {
            coin: key,
            price_usd: val.price,
            symbol: val.symbol,
            decimals: val.decimals,
            timestamp: val.timestamp,
            confidence: val.confidence,
        })
        .collect();
    prices.sort_by(|a, b| a.coin.cmp(&b.coin));
    dto::CoinsPriceDto { prices }
}

pub fn map_coins_change(
    raw: client::RawCoinsChangeResponse,
    period: Option<&str>,
) -> dto::CoinsChangeDto {
    let mut coins: Vec<dto::CoinChangeEntry> = raw
        .coins
        .into_iter()
        .map(|(key, pct)| dto::CoinChangeEntry {
            coin: key,
            change_pct: pct,
        })
        .collect();
    coins.sort_by(|a, b| a.coin.cmp(&b.coin));
    dto::CoinsChangeDto {
        period: period.map(|s| s.to_owned()),
        coins,
    }
}

pub fn map_block(raw: client::RawBlockResponse, chain: &str) -> dto::BlockDto {
    dto::BlockDto {
        chain: chain.to_owned(),
        height: raw.height,
        timestamp: raw.timestamp,
    }
}

pub fn map_coins_liquidity(
    raw: Vec<client::RawLiquidityPoint>,
    token: &str,
) -> dto::CoinLiquidityDto {
    let points = raw
        .into_iter()
        .filter_map(|p| {
            Some(dto::LiquidityPoint {
                date: p.date?,
                liquidity_usd: p.liquidity?,
            })
        })
        .collect();
    dto::CoinLiquidityDto {
        token: token.to_owned(),
        points,
    }
}

pub fn map_coins_chart(raw: client::RawCoinChart) -> dto::CoinsChartDto {
    let mut coins: Vec<dto::CoinChartEntry> = raw
        .coins
        .into_iter()
        .map(|(key, data)| dto::CoinChartEntry {
            coin: key,
            symbol: data.symbol,
            points: data
                .prices
                .into_iter()
                .filter_map(|p| {
                    Some(dto::ChartPoint {
                        timestamp: p.timestamp?,
                        price_usd: p.price?,
                    })
                })
                .collect(),
        })
        .collect();
    coins.sort_by(|a, b| a.coin.cmp(&b.coin));
    dto::CoinsChartDto { coins }
}

// ---------------------------------------------------------------------------
// Stablecoins
// ---------------------------------------------------------------------------

fn extract_circulating_usd(val: &Option<serde_json::Value>) -> Option<f64> {
    val.as_ref()
        .and_then(|v| v.get("peggedUSD"))
        .and_then(|v| v.as_f64())
}

pub fn map_stablecoins(raw: client::RawStablecoinsResponse) -> dto::StablecoinsListDto {
    let stablecoins = raw
        .pegged_assets
        .into_iter()
        .map(|s| dto::StablecoinRow {
            id: s.id.unwrap_or_default(),
            name: s.name.unwrap_or_default(),
            symbol: s.symbol.unwrap_or_default(),
            peg_type: s.peg_type,
            peg_mechanism: s.peg_mechanism,
            price: s.price,
            circulating_usd: extract_circulating_usd(&s.circulating),
            chains: s.chains,
        })
        .collect();
    dto::StablecoinsListDto { stablecoins }
}

pub fn map_stablecoin_detail(raw: client::RawStablecoinDetail) -> dto::StablecoinDetailDto {
    // currentChainBalances: { "Ethereum": { "peggedUSD": 80000000000 }, ... }
    let chain_circulating: std::collections::HashMap<String, f64> = raw
        .current_chain_balances
        .as_ref()
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(chain, balance)| {
                    let circ = balance.get("peggedUSD").and_then(|v| v.as_f64())?;
                    Some((chain.clone(), circ))
                })
                .collect()
        })
        .unwrap_or_default();

    // Derive chains from currentChainBalances keys.
    let chains: Vec<String> = chain_circulating.keys().cloned().collect();

    dto::StablecoinDetailDto {
        id: raw.id.unwrap_or_default(),
        name: raw.name.unwrap_or_default(),
        symbol: raw.symbol.unwrap_or_default(),
        peg_type: raw.peg_type,
        peg_mechanism: raw.peg_mechanism,
        price: raw.price,
        chains,
        chain_circulating,
    }
}

/// Parse date which can be a string or number in upstream data.
fn parse_date_value(val: &Option<serde_json::Value>) -> Option<i64> {
    match val {
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

pub fn map_stablecoin_chains(raw: Vec<client::RawStablecoinChain>) -> dto::StablecoinChainsDto {
    let chains = raw
        .into_iter()
        .filter_map(|c| {
            Some(dto::StablecoinChainRow {
                name: c.name?,
                gecko_id: c.gecko_id,
                circulating_usd: extract_circulating_usd(&c.total_circulating_usd),
            })
        })
        .collect();
    dto::StablecoinChainsDto { chains }
}

pub fn map_stablecoin_chart(
    raw: Vec<client::RawStablecoinChartPoint>,
    chain: Option<&str>,
) -> dto::StablecoinChartDto {
    let points = raw
        .into_iter()
        .filter_map(|p| {
            let date = parse_date_value(&p.date)?;
            let circ = extract_circulating_usd(&p.total_circulating_usd)
                .or_else(|| extract_circulating_usd(&p.total_circulating))?;
            Some(dto::StablecoinChartPoint {
                date,
                circulating_usd: circ,
            })
        })
        .collect();
    dto::StablecoinChartDto {
        chain: chain.map(|s| s.to_owned()),
        points,
    }
}

pub fn map_stablecoin_dominance(
    raw: Vec<client::RawStablecoinDominancePoint>,
    chain: &str,
) -> dto::StablecoinDominanceDto {
    let points = raw
        .into_iter()
        .filter_map(|p| {
            let date = p.date?;
            let total = extract_circulating_usd(&p.total_circulating);
            let dominance = p
                .dominance
                .as_ref()
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(name, pct)| {
                            Some(dto::StablecoinDominanceEntry {
                                name: name.clone(),
                                dominance_pct: pct.as_f64()?,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(dto::StablecoinDominancePoint {
                date,
                total_circulating_usd: total,
                dominance,
            })
        })
        .collect();
    dto::StablecoinDominanceDto {
        chain: chain.to_owned(),
        points,
    }
}

pub fn map_stablecoin_prices(
    raw: Vec<client::RawStablecoinPricePoint>,
) -> dto::StablecoinPricesDto {
    let points = raw
        .into_iter()
        .filter_map(|p| {
            let date = p.date?;
            let prices = p
                .prices
                .as_ref()
                .and_then(|v| v.as_object())
                .map(|obj| {
                    let mut entries: Vec<dto::StablecoinPriceEntry> = obj
                        .iter()
                        .filter_map(|(name, price)| {
                            Some(dto::StablecoinPriceEntry {
                                name: name.clone(),
                                price: price.as_f64()?,
                            })
                        })
                        .collect();
                    entries.sort_by(|a, b| a.name.cmp(&b.name));
                    entries
                })
                .unwrap_or_default();
            Some(dto::StablecoinPricesPoint { date, prices })
        })
        .collect();
    dto::StablecoinPricesDto { points }
}

// ---------------------------------------------------------------------------
// Bridges
// ---------------------------------------------------------------------------

pub fn map_bridges(raw: client::RawBridgesResponse) -> dto::BridgesListDto {
    let bridges = raw
        .bridges
        .into_iter()
        .map(|b| dto::BridgeRow {
            id: b.id.unwrap_or(0),
            name: b.display_name.or(b.name).unwrap_or_default(),
            daily_volume_usd: b.last_daily_volume.or(b.current_day_volume),
            weekly_volume_usd: b.weekly_volume,
            monthly_volume_usd: b.monthly_volume,
            chains: b.chains,
        })
        .collect();
    dto::BridgesListDto { bridges }
}

pub fn map_bridge_detail(raw: client::RawBridgeDetail) -> dto::BridgeDetailDto {
    dto::BridgeDetailDto {
        id: raw.id.unwrap_or(0),
        name: raw.display_name.or(raw.name).unwrap_or_default(),
        chains: raw.chains,
        destination_chain: raw.destination_chain,
    }
}

pub fn map_bridge_volume(
    raw: Vec<client::RawBridgeVolumePoint>,
    chain: &str,
) -> dto::BridgeVolumeDto {
    let points = raw
        .into_iter()
        .filter_map(|p| {
            Some(dto::BridgeVolumePoint {
                date: p.date?,
                deposit_usd: p.deposit_usd,
                withdraw_usd: p.withdraw_usd,
                deposit_txs: p.deposit_txs,
                withdraw_txs: p.withdraw_txs,
            })
        })
        .collect();
    dto::BridgeVolumeDto {
        chain: chain.to_owned(),
        points,
    }
}

fn count_json_keys(val: &Option<serde_json::Value>) -> usize {
    val.as_ref()
        .and_then(|v| v.as_object())
        .map(|o| o.len())
        .unwrap_or(0)
}

pub fn map_bridge_daystats(
    raw: Vec<client::RawBridgeDayStats>,
    chain: &str,
    timestamp: i64,
) -> dto::BridgeDayStatsDto {
    let stats = raw
        .into_iter()
        .filter_map(|s| {
            Some(dto::BridgeDayStatEntry {
                date: s.date?,
                tokens_deposited_count: count_json_keys(&s.total_tokens_deposited),
                tokens_withdrawn_count: count_json_keys(&s.total_tokens_withdrawn),
                addresses_deposited_count: count_json_keys(&s.total_address_deposited),
                addresses_withdrawn_count: count_json_keys(&s.total_address_withdrawn),
            })
        })
        .collect();
    dto::BridgeDayStatsDto {
        chain: chain.to_owned(),
        timestamp,
        stats,
    }
}

pub fn map_bridge_transactions(raw: Vec<client::RawBridgeTx>, bridge_id: u64) -> dto::BridgeTxDto {
    let transactions = raw
        .into_iter()
        .filter_map(|tx| {
            Some(dto::BridgeTxRow {
                tx_hash: tx.tx_hash?,
                timestamp: tx.ts,
                chain: tx.chain,
                token: tx.token,
                amount: tx.amount,
                is_deposit: tx.is_deposit,
                from: tx.tx_from,
                to: tx.tx_to,
            })
        })
        .collect();
    dto::BridgeTxDto {
        bridge_id,
        transactions,
    }
}

// ---------------------------------------------------------------------------
// Yields
// ---------------------------------------------------------------------------

pub fn map_yield_pools(raw: client::RawYieldsResponse<client::RawYieldPool>) -> dto::YieldPoolsDto {
    let pools = raw
        .data
        .into_iter()
        .filter_map(|p| {
            Some(dto::YieldPoolRow {
                pool: p.pool?,
                chain: p.chain,
                project: p.project,
                symbol: p.symbol,
                tvl_usd: p.tvl_usd,
                apy: p.apy,
                apy_base: p.apy_base,
                apy_reward: p.apy_reward,
                stablecoin: p.stablecoin,
                il_risk: p.il_risk,
                exposure: p.exposure,
            })
        })
        .collect();
    dto::YieldPoolsDto { pools }
}

pub fn map_yield_borrow_pools(
    raw: client::RawYieldsResponse<client::RawYieldBorrowPool>,
) -> dto::YieldBorrowPoolsDto {
    let pools = raw
        .data
        .into_iter()
        .filter_map(|p| {
            Some(dto::YieldBorrowPoolRow {
                pool: p.pool?,
                chain: p.chain,
                project: p.project,
                symbol: p.symbol,
                tvl_usd: p.tvl_usd,
                apy: p.apy,
                apy_base: p.apy_base,
                apy_reward: p.apy_reward,
                apy_base_borrow: p.apy_base_borrow,
                apy_reward_borrow: p.apy_reward_borrow,
                total_supply_usd: p.total_supply_usd,
                total_borrow_usd: p.total_borrow_usd,
                stablecoin: p.stablecoin,
            })
        })
        .collect();
    dto::YieldBorrowPoolsDto { pools }
}

pub fn map_yield_chart(
    raw: client::RawYieldsResponse<client::RawYieldChartPoint>,
    pool: &str,
) -> dto::YieldChartDto {
    let points = raw
        .data
        .into_iter()
        .filter_map(|p| {
            Some(dto::YieldChartPoint {
                timestamp: p.timestamp?,
                tvl_usd: p.tvl_usd,
                apy: p.apy,
                apy_base: p.apy_base,
                apy_reward: p.apy_reward,
            })
        })
        .collect();
    dto::YieldChartDto {
        pool: pool.to_owned(),
        points,
    }
}

pub fn map_yield_lend_borrow_chart(
    raw: client::RawYieldsResponse<client::RawYieldLendBorrowChartPoint>,
    pool: &str,
) -> dto::YieldLendBorrowChartDto {
    let points = raw
        .data
        .into_iter()
        .filter_map(|p| {
            Some(dto::YieldLendBorrowChartPoint {
                timestamp: p.timestamp?,
                tvl_usd: p.tvl_usd,
                apy: p.apy,
                apy_base: p.apy_base,
                apy_reward: p.apy_reward,
                apy_base_borrow: p.apy_base_borrow,
                apy_reward_borrow: p.apy_reward_borrow,
                total_supply_usd: p.total_supply_usd,
                total_borrow_usd: p.total_borrow_usd,
            })
        })
        .collect();
    dto::YieldLendBorrowChartDto {
        pool: pool.to_owned(),
        points,
    }
}

pub fn map_perps(raw: client::RawYieldsResponse<client::RawPerp>) -> dto::PerpsDto {
    let perps = raw
        .data
        .into_iter()
        .map(|p| dto::PerpRow {
            marketplace: p.marketplace,
            symbol: p.symbol,
            base_asset: p.base_asset,
            funding_rate: p.funding_rate,
            open_interest: p.open_interest,
            index_price: p.index_price,
        })
        .collect();
    dto::PerpsDto { perps }
}

pub fn map_lsd(raw: client::RawYieldsResponse<client::RawLsdRate>) -> dto::LsdDto {
    let rates = raw
        .data
        .into_iter()
        .filter_map(|r| {
            Some(dto::LsdRow {
                name: r.name?,
                symbol: r.symbol,
                eth_peg: r.eth_peg,
                apy: r.apy,
                market_share: r.market_share,
                fee: r.fee,
            })
        })
        .collect();
    dto::LsdDto { rates }
}

// ---------------------------------------------------------------------------
// Ecosystem / Intelligence
// ---------------------------------------------------------------------------

pub fn map_categories(raw: client::RawCategoriesResponse) -> dto::CategoriesDto {
    let mut categories: Vec<dto::CategoryRow> = raw
        .categories
        .into_iter()
        .map(|(name, protocols)| dto::CategoryRow {
            protocol_count: protocols.len(),
            name,
        })
        .collect();
    categories.sort_by(|a, b| b.protocol_count.cmp(&a.protocol_count));
    dto::CategoriesDto { categories }
}

pub fn map_forks(raw: client::RawForksResponse) -> dto::ForksDto {
    let mut forks: Vec<dto::ForkRow> = raw
        .forks
        .into_iter()
        .map(|(name, entry)| dto::ForkRow {
            name,
            tvl_usd: entry.tvl,
            fork_count: entry.forked_protocols.len(),
        })
        .collect();
    forks.sort_by(|a, b| {
        b.tvl_usd
            .unwrap_or(0.0)
            .partial_cmp(&a.tvl_usd.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    dto::ForksDto { forks }
}

pub fn map_oracles(raw: client::RawOraclesResponse) -> dto::OraclesDto {
    let mut oracles: Vec<dto::OracleRow> = raw
        .oracles
        .into_iter()
        .map(|(name, tvl)| dto::OracleRow {
            name,
            tvl_secured_usd: tvl,
        })
        .collect();
    oracles.sort_by(|a, b| {
        b.tvl_secured_usd
            .partial_cmp(&a.tvl_secured_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    dto::OraclesDto { oracles }
}

pub fn map_entities(raw: Vec<client::RawEntity>) -> dto::EntitiesDto {
    let entities = raw
        .into_iter()
        .filter_map(|e| {
            Some(dto::EntityRow {
                name: e.name?,
                category: e.category,
                tvl_usd: e.tvl,
                change_1d_pct: e.change_1d,
                change_7d_pct: e.change_7d,
                chains: e.chains,
            })
        })
        .collect();
    dto::EntitiesDto { entities }
}

pub fn map_raises(raw: client::RawRaisesResponse) -> dto::RaisesDto {
    let raises = raw
        .raises
        .into_iter()
        .filter_map(|r| {
            Some(dto::RaiseRow {
                name: r.name?,
                round: r.round,
                amount_usd: r.amount,
                date: r.date,
                lead_investors: r.lead_investors,
                category: r.category,
                chains: r.chains,
            })
        })
        .collect();
    dto::RaisesDto { raises }
}

pub fn map_treasuries(raw: Vec<client::RawTreasury>) -> dto::TreasuriesDto {
    let treasuries = raw
        .into_iter()
        .filter_map(|t| {
            Some(dto::TreasuryRow {
                name: t.name?,
                symbol: t.symbol,
                category: t.category,
                tvl_usd: t.tvl,
                change_1d_pct: t.change_1d,
                change_7d_pct: t.change_7d,
            })
        })
        .collect();
    dto::TreasuriesDto { treasuries }
}

pub fn map_hacks(raw: Vec<client::RawHack>) -> dto::HacksDto {
    let hacks = raw
        .into_iter()
        .filter_map(|h| {
            Some(dto::HackRow {
                name: h.name?,
                date: h.date,
                amount_usd: h.amount,
                chains: h.chain,
                classification: h.classification,
                technique: h.technique,
                target_type: h.target_type,
            })
        })
        .collect();
    dto::HacksDto { hacks }
}

pub fn map_inflows(raw: client::RawInflowsResponse, protocol: &str) -> dto::InflowsDto {
    dto::InflowsDto {
        protocol: protocol.to_owned(),
        outflows_usd: raw.outflows,
        old_tokens_date: raw.old_tokens.as_ref().and_then(|t| t.date),
        current_tokens_date: raw.current_tokens.as_ref().and_then(|t| t.date),
        old_tokens: raw.old_tokens.map(|t| t.tvl).unwrap_or_default(),
        current_tokens: raw.current_tokens.map(|t| t.tvl).unwrap_or_default(),
    }
}

pub fn map_token_protocols(
    raw: Vec<client::RawTokenProtocol>,
    symbol: &str,
) -> dto::TokenProtocolsDto {
    let protocols = raw
        .into_iter()
        .filter_map(|p| {
            let total: f64 = p.amount_usd.values().sum();
            Some(dto::TokenProtocolRow {
                name: p.name?,
                category: p.category,
                total_amount_usd: if total > 0.0 { Some(total) } else { None },
            })
        })
        .collect();
    dto::TokenProtocolsDto {
        symbol: symbol.to_owned(),
        protocols,
    }
}

// ---------------------------------------------------------------------------
// Fees / Volumes
// ---------------------------------------------------------------------------

pub fn map_protocol_summary(
    raw: client::RawSummaryResponse,
    metric: &str,
) -> dto::ProtocolSummaryDto {
    dto::ProtocolSummaryDto {
        metric: metric.to_owned(),
        name: raw.name.unwrap_or_default(),
        slug: raw.slug,
        category: raw.category,
        chains: raw.chains,
        total_24h_usd: raw.total24h,
        total_7d_usd: raw.total7d,
        total_30d_usd: raw.total30d,
        change_1d_pct: raw.change_1d,
        change_7d_pct: raw.change_7d,
    }
}

pub fn map_overview(
    raw: client::RawOverviewResponse,
    metric: &str,
    chain: Option<&str>,
) -> dto::OverviewDto {
    let protocols = raw
        .protocols
        .into_iter()
        .map(|p| dto::OverviewProtocolRow {
            name: p.name.unwrap_or_default(),
            slug: p.slug,
            category: p.category,
            total_24h_usd: p.total24h,
            total_7d_usd: p.total7d,
            change_1d_pct: p.change_1d,
            change_7d_pct: p.change_7d,
            chains: p.chains,
        })
        .collect();

    dto::OverviewDto {
        metric: metric.to_owned(),
        chain: chain.map(|s| s.to_owned()),
        total_24h_usd: raw.total24h,
        total_7d_usd: raw.total7d,
        change_1d_pct: raw.change_1d,
        change_7d_pct: raw.change_7d,
        protocols,
    }
}

// ---------------------------------------------------------------------------
// Institutions
// ---------------------------------------------------------------------------

pub fn map_institutions(raw: client::RawInstitutionsResponse) -> dto::InstitutionsDto {
    let institutions = raw
        .institutions
        .into_iter()
        .filter_map(|entry| {
            let id_str = entry.institution_id?.to_string();
            let meta = raw.institution_metadata.get(&id_str);
            Some(dto::InstitutionRow {
                name: meta.and_then(|m| m.name.clone()).unwrap_or_else(|| id_str),
                ticker: meta.and_then(|m| m.ticker.clone()),
                inst_type: meta.and_then(|m| m.inst_type.clone()),
                total_value_usd: entry.total_usd_value,
                total_cost_usd: entry.total_cost,
            })
        })
        .collect();
    dto::InstitutionsDto { institutions }
}

pub fn map_institution_detail(raw: client::RawInstitutionDetail) -> dto::InstitutionDetailDto {
    dto::InstitutionDetailDto {
        name: raw.name.unwrap_or_default(),
        ticker: raw.ticker,
        inst_type: raw.inst_type,
        price: raw.price,
        total_value_usd: raw.total_usd_value,
        total_cost_usd: raw.total_cost,
    }
}

// ---------------------------------------------------------------------------
// ETFs
// ---------------------------------------------------------------------------

pub fn map_etf_snapshot(raw: Vec<client::RawEtfSnapshot>) -> dto::EtfSnapshotDto {
    let etfs = raw
        .into_iter()
        .map(|e| dto::EtfSnapshotRow {
            ticker: e.ticker,
            name: e.etf_name,
            issuer: e.issuer,
            asset: e.asset,
            fee_pct: e.pct_fee,
            flows_usd: e.flows,
            aum_usd: e.aum,
            volume: e.volume,
        })
        .collect();
    dto::EtfSnapshotDto { etfs }
}

pub fn map_etf_flows(raw: Vec<client::RawEtfFlow>) -> dto::EtfFlowsDto {
    let flows = raw
        .into_iter()
        .filter_map(|f| {
            Some(dto::EtfFlowPoint {
                date: f.day?,
                gecko_id: f.gecko_id,
                total_flow_usd: f.total_flow_usd,
            })
        })
        .collect();
    dto::EtfFlowsDto { flows }
}

// ---------------------------------------------------------------------------
// FDV
// ---------------------------------------------------------------------------

pub fn map_fdv_performance(raw: Vec<serde_json::Value>, period: &str) -> dto::FdvPerformanceDto {
    let points = raw
        .into_iter()
        .filter_map(|val| {
            let obj = val.as_object()?;
            let date = obj.get("date").and_then(|v| v.as_i64());
            let categories: Vec<dto::FdvCategoryEntry> = obj
                .iter()
                .filter(|(k, _)| *k != "date")
                .filter_map(|(k, v)| {
                    Some(dto::FdvCategoryEntry {
                        category: k.clone(),
                        performance: v.as_f64()?,
                    })
                })
                .collect();
            if categories.is_empty() {
                return None;
            }
            Some(dto::FdvPerformancePoint { date, categories })
        })
        .collect();
    dto::FdvPerformanceDto {
        period: period.to_owned(),
        points,
    }
}

// ---------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------

pub fn map_usage(raw: client::RawUsage) -> dto::UsageDto {
    dto::UsageDto {
        requests_today: raw.requests_today,
        requests_this_month: raw.requests_this_month,
        rate_limit: raw.rate_limit,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_protocols_empty() {
        let dto = map_protocols(vec![]);
        assert!(dto.protocols.is_empty());
    }

    #[test]
    fn map_protocols_basic() {
        let raw = vec![client::RawProtocol {
            id: Some("1".into()),
            name: Some("Aave".into()),
            symbol: Some("AAVE".into()),
            slug: Some("aave".into()),
            category: Some("Lending".into()),
            chains: vec!["Ethereum".into()],
            tvl: Some(5_000_000_000.0),
            change_1d: Some(2.1),
            change_7d: Some(-5.3),
            change_1m: None,
            url: Some("https://aave.com".into()),
            logo: None,
        }];
        let dto = map_protocols(raw);
        assert_eq!(dto.protocols.len(), 1);
        assert_eq!(dto.protocols[0].slug, "aave");
        assert_eq!(dto.protocols[0].tvl_usd, Some(5_000_000_000.0));
    }

    #[test]
    fn map_chains_empty() {
        let dto = map_chains(vec![]);
        assert!(dto.chains.is_empty());
    }

    #[test]
    fn map_chain_history_filters_none() {
        let raw = vec![
            client::RawTvlPoint {
                date: Some(1000),
                tvl: Some(100.0),
                total_liquidity_usd: None,
            },
            client::RawTvlPoint {
                date: None,
                tvl: Some(200.0),
                total_liquidity_usd: None,
            },
        ];
        let dto = map_chain_history(raw, Some("ethereum"));
        assert_eq!(dto.points.len(), 1);
        assert_eq!(dto.chain, Some("ethereum".to_owned()));
    }

    #[test]
    fn map_coins_price_sorts_by_key() {
        let mut coins = std::collections::HashMap::new();
        coins.insert(
            "ethereum:0xbbb".to_string(),
            client::RawCoinPrice {
                price: Some(1.0),
                symbol: Some("B".into()),
                decimals: None,
                timestamp: None,
                confidence: None,
            },
        );
        coins.insert(
            "ethereum:0xaaa".to_string(),
            client::RawCoinPrice {
                price: Some(2.0),
                symbol: Some("A".into()),
                decimals: None,
                timestamp: None,
                confidence: None,
            },
        );
        let dto = map_coins_price(client::RawCoinsResponse { coins });
        assert_eq!(dto.prices[0].coin, "ethereum:0xaaa");
        assert_eq!(dto.prices[1].coin, "ethereum:0xbbb");
    }

    #[test]
    fn extract_circulating_handles_none() {
        assert_eq!(extract_circulating_usd(&None), None);
    }

    #[test]
    fn extract_circulating_handles_pegged_usd() {
        let val = serde_json::json!({"peggedUSD": 83000000000.0});
        assert_eq!(extract_circulating_usd(&Some(val)), Some(83_000_000_000.0));
    }

    #[test]
    fn map_stablecoin_detail_current_chain_balances() {
        let raw = client::RawStablecoinDetail {
            id: Some("1".into()),
            name: Some("Tether".into()),
            symbol: Some("USDT".into()),
            gecko_id: None,
            peg_type: Some("peggedUSD".into()),
            peg_mechanism: Some("fiat-backed".into()),
            price: Some(1.0001),
            price_source: None,
            current_chain_balances: Some(serde_json::json!({
                "Ethereum": { "peggedUSD": 80_000_000_000.0 },
                "Tron": { "peggedUSD": 60_000_000_000.0 }
            })),
        };
        let dto = map_stablecoin_detail(raw);
        assert_eq!(dto.name, "Tether");
        assert_eq!(dto.chain_circulating.len(), 2);
        assert_eq!(dto.chain_circulating["Ethereum"], 80_000_000_000.0);
        assert_eq!(dto.chain_circulating["Tron"], 60_000_000_000.0);
        assert_eq!(dto.chains.len(), 2);
    }

    #[test]
    fn map_stablecoin_detail_no_chain_balances() {
        let raw = client::RawStablecoinDetail {
            id: Some("2".into()),
            name: Some("DAI".into()),
            symbol: Some("DAI".into()),
            gecko_id: None,
            peg_type: None,
            peg_mechanism: None,
            price: None,
            price_source: None,
            current_chain_balances: None,
        };
        let dto = map_stablecoin_detail(raw);
        assert!(dto.chain_circulating.is_empty());
        assert!(dto.chains.is_empty());
    }

    #[test]
    fn map_coins_change_sorts() {
        let mut coins = std::collections::HashMap::new();
        coins.insert("coingecko:ethereum".to_string(), -2.5);
        coins.insert("coingecko:bitcoin".to_string(), 5.3);
        let dto = map_coins_change(client::RawCoinsChangeResponse { coins }, Some("7d"));
        assert_eq!(dto.coins[0].coin, "coingecko:bitcoin");
        assert_eq!(dto.coins[1].coin, "coingecko:ethereum");
        assert_eq!(dto.period, Some("7d".to_owned()));
    }

    #[test]
    fn map_block_basic() {
        let raw = client::RawBlockResponse {
            height: Some(12345678),
            timestamp: Some(1700000000),
        };
        let dto = map_block(raw, "ethereum");
        assert_eq!(dto.chain, "ethereum");
        assert_eq!(dto.height, Some(12345678));
    }

    #[test]
    fn map_coins_liquidity_filters_none() {
        let raw = vec![
            client::RawLiquidityPoint {
                date: Some(1000),
                liquidity: Some(500.0),
            },
            client::RawLiquidityPoint {
                date: None,
                liquidity: Some(600.0),
            },
            client::RawLiquidityPoint {
                date: Some(2000),
                liquidity: None,
            },
        ];
        let dto = map_coins_liquidity(raw, "ethereum:0xabc");
        assert_eq!(dto.points.len(), 1);
        assert_eq!(dto.token, "ethereum:0xabc");
    }

    #[test]
    fn map_protocol_detail_preserves_requested_slug() {
        let raw = client::RawProtocolDetail {
            id: Some("1".into()),
            name: Some("Aave".into()),
            symbol: Some("AAVE".into()),
            slug: None, // upstream doesn't include it
            category: Some("Lending".into()),
            description: None,
            url: None,
            logo: None,
            chains: vec!["Ethereum".into()],
            tvl: Some(serde_json::json!(5_000_000_000.0)),
            current_chain_tvls: std::collections::HashMap::new(),
            mcap: None,
        };
        let dto = map_protocol_detail(raw, "aave");
        assert_eq!(dto.slug, "aave");
    }

    #[test]
    fn map_protocol_detail_uses_upstream_slug_when_present() {
        let raw = client::RawProtocolDetail {
            id: Some("1".into()),
            name: Some("Aave".into()),
            symbol: None,
            slug: Some("aave-v3".into()),
            category: None,
            description: None,
            url: None,
            logo: None,
            chains: vec![],
            tvl: None,
            current_chain_tvls: std::collections::HashMap::new(),
            mcap: None,
        };
        let dto = map_protocol_detail(raw, "aave");
        assert_eq!(dto.slug, "aave-v3"); // upstream slug wins when present
    }

    #[test]
    fn map_bridges_empty() {
        let dto = map_bridges(client::RawBridgesResponse { bridges: vec![] });
        assert!(dto.bridges.is_empty());
    }

    #[test]
    fn map_stablecoin_chains_basic() {
        let raw = vec![client::RawStablecoinChain {
            gecko_id: Some("ethereum".into()),
            name: Some("Ethereum".into()),
            total_circulating_usd: Some(serde_json::json!({"peggedUSD": 80_000_000_000.0})),
        }];
        let dto = map_stablecoin_chains(raw);
        assert_eq!(dto.chains.len(), 1);
        assert_eq!(dto.chains[0].name, "Ethereum");
        assert_eq!(dto.chains[0].circulating_usd, Some(80_000_000_000.0));
    }

    #[test]
    fn map_stablecoin_chart_parses_string_dates() {
        let raw = vec![client::RawStablecoinChartPoint {
            date: Some(serde_json::json!("1700000000")),
            total_circulating: None,
            total_circulating_usd: Some(serde_json::json!({"peggedUSD": 130_000_000_000.0})),
        }];
        let dto = map_stablecoin_chart(raw, None);
        assert_eq!(dto.points.len(), 1);
        assert_eq!(dto.points[0].date, 1700000000);
        assert_eq!(dto.points[0].circulating_usd, 130_000_000_000.0);
    }

    #[test]
    fn map_stablecoin_prices_sorts() {
        let raw = vec![client::RawStablecoinPricePoint {
            date: Some(1700000000),
            prices: Some(serde_json::json!({"USDT": 1.0001, "DAI": 0.9999, "USDC": 1.0000})),
        }];
        let dto = map_stablecoin_prices(raw);
        assert_eq!(dto.points.len(), 1);
        assert_eq!(dto.points[0].prices[0].name, "DAI");
        assert_eq!(dto.points[0].prices[1].name, "USDC");
        assert_eq!(dto.points[0].prices[2].name, "USDT");
    }

    #[test]
    fn map_bridge_volume_filters_none_dates() {
        let raw = vec![
            client::RawBridgeVolumePoint {
                date: Some(1000),
                deposit_usd: Some(100.0),
                withdraw_usd: Some(50.0),
                deposit_txs: Some(10),
                withdraw_txs: Some(5),
            },
            client::RawBridgeVolumePoint {
                date: None,
                deposit_usd: Some(200.0),
                withdraw_usd: None,
                deposit_txs: None,
                withdraw_txs: None,
            },
        ];
        let dto = map_bridge_volume(raw, "ethereum");
        assert_eq!(dto.points.len(), 1);
        assert_eq!(dto.chain, "ethereum");
    }

    #[test]
    fn map_bridge_transactions_filters_no_hash() {
        let raw = vec![
            client::RawBridgeTx {
                tx_hash: Some("0xabc".into()),
                ts: Some(1700000000),
                tx_block: Some(12345),
                tx_from: Some("0xfrom".into()),
                tx_to: Some("0xto".into()),
                token: Some("USDC".into()),
                amount: Some("1000000".into()),
                is_deposit: Some(true),
                chain: Some("ethereum".into()),
            },
            client::RawBridgeTx {
                tx_hash: None, // should be filtered out
                ts: Some(1700000000),
                tx_block: None,
                tx_from: None,
                tx_to: None,
                token: None,
                amount: None,
                is_deposit: None,
                chain: None,
            },
        ];
        let dto = map_bridge_transactions(raw, 1);
        assert_eq!(dto.transactions.len(), 1);
        assert_eq!(dto.transactions[0].tx_hash, "0xabc");
    }

    #[test]
    fn map_yield_pools_filters_no_pool_id() {
        let raw = client::RawYieldsResponse {
            status: Some("success".into()),
            data: vec![
                client::RawYieldPool {
                    chain: Some("Ethereum".into()),
                    project: Some("aave-v3".into()),
                    symbol: Some("USDC".into()),
                    pool: Some("pool-1".into()),
                    tvl_usd: Some(1000.0),
                    apy: Some(3.5),
                    apy_base: Some(2.0),
                    apy_reward: Some(1.5),
                    stablecoin: Some(true),
                    il_risk: None,
                    exposure: None,
                    pool_meta: None,
                },
                client::RawYieldPool {
                    chain: Some("BSC".into()),
                    project: None,
                    symbol: None,
                    pool: None, // should be filtered
                    tvl_usd: None,
                    apy: None,
                    apy_base: None,
                    apy_reward: None,
                    stablecoin: None,
                    il_risk: None,
                    exposure: None,
                    pool_meta: None,
                },
            ],
        };
        let dto = map_yield_pools(raw);
        assert_eq!(dto.pools.len(), 1);
        assert_eq!(dto.pools[0].pool, "pool-1");
    }

    #[test]
    fn map_perps_basic() {
        let raw = client::RawYieldsResponse {
            status: Some("success".into()),
            data: vec![client::RawPerp {
                marketplace: Some("Hyperliquid".into()),
                symbol: Some("ETH".into()),
                funding_rate: Some(0.001),
                open_interest: Some(500_000_000.0),
                index_price: Some(3500.0),
                base_asset: Some("ETH".into()),
            }],
        };
        let dto = map_perps(raw);
        assert_eq!(dto.perps.len(), 1);
        assert_eq!(dto.perps[0].marketplace, Some("Hyperliquid".to_owned()));
    }

    #[test]
    fn map_lsd_filters_no_name() {
        let raw = client::RawYieldsResponse {
            status: Some("success".into()),
            data: vec![
                client::RawLsdRate {
                    name: Some("Lido".into()),
                    symbol: Some("stETH".into()),
                    eth_peg: Some(0.9998),
                    apy: Some(3.2),
                    market_share: Some(70.0),
                    fee: Some(10.0),
                },
                client::RawLsdRate {
                    name: None,
                    symbol: None,
                    eth_peg: None,
                    apy: None,
                    market_share: None,
                    fee: None,
                },
            ],
        };
        let dto = map_lsd(raw);
        assert_eq!(dto.rates.len(), 1);
        assert_eq!(dto.rates[0].name, "Lido");
    }

    #[test]
    fn map_overview_empty() {
        let raw = client::RawOverviewResponse {
            total24h: Some(1000.0),
            total48hto24h: None,
            total7d: None,
            total30d: None,
            total1y: None,
            change_1d: Some(5.0),
            change_7d: None,
            change_1m: None,
            protocols: vec![],
        };
        let dto = map_overview(raw, "fees", None);
        assert_eq!(dto.metric, "fees");
        assert_eq!(dto.total_24h_usd, Some(1000.0));
        assert!(dto.protocols.is_empty());
    }

    #[test]
    fn map_protocol_summary_basic() {
        let raw = client::RawSummaryResponse {
            name: Some("Uniswap".into()),
            slug: Some("uniswap".into()),
            category: Some("DEX".into()),
            chains: vec!["Ethereum".into()],
            total24h: Some(5_000_000.0),
            total48hto24h: None,
            total7d: Some(30_000_000.0),
            total30d: Some(120_000_000.0),
            total1y: None,
            change_1d: Some(3.5),
            change_7d: Some(-1.2),
            change_1m: None,
            chain_data: None,
        };
        let dto = map_protocol_summary(raw, "dex_volume");
        assert_eq!(dto.name, "Uniswap");
        assert_eq!(dto.metric, "dex_volume");
        assert_eq!(dto.total_24h_usd, Some(5_000_000.0));
    }

    #[test]
    fn map_categories_from_hashmap() {
        let mut categories = std::collections::HashMap::new();
        categories.insert(
            "Lending".to_string(),
            vec!["Aave".into(), "Compound".into()],
        );
        categories.insert("DEX".to_string(), vec!["Uniswap".into()]);
        let raw = client::RawCategoriesResponse { categories };
        let dto = map_categories(raw);
        assert_eq!(dto.categories.len(), 2);
        // Sorted by protocol_count descending
        assert_eq!(dto.categories[0].name, "Lending");
        assert_eq!(dto.categories[0].protocol_count, 2);
        assert_eq!(dto.categories[1].name, "DEX");
        assert_eq!(dto.categories[1].protocol_count, 1);
    }

    #[test]
    fn map_forks_sorts_by_tvl() {
        let mut forks = std::collections::HashMap::new();
        forks.insert(
            "A".to_string(),
            client::RawForkEntry {
                forked_protocols: vec!["x".into()],
                tvl: Some(100.0),
            },
        );
        forks.insert(
            "B".to_string(),
            client::RawForkEntry {
                forked_protocols: vec!["y".into(), "z".into()],
                tvl: Some(200.0),
            },
        );
        let dto = map_forks(client::RawForksResponse { forks });
        assert_eq!(dto.forks[0].name, "B"); // higher TVL first
        assert_eq!(dto.forks[0].fork_count, 2);
    }

    #[test]
    fn map_hacks_filters_no_name() {
        let raw = vec![
            client::RawHack {
                name: Some("Hack1".into()),
                date: Some(1672531200),
                amount: Some(100_000_000.0),
                chain: vec!["Ethereum".into()],
                classification: Some("Exploit".into()),
                technique: None,
                target_type: Some("DeFi Protocol".into()),
                bridge_hack: Some(false),
            },
            client::RawHack {
                name: None,
                date: None,
                amount: None,
                chain: vec![],
                classification: None,
                technique: None,
                target_type: None,
                bridge_hack: None,
            },
        ];
        let dto = map_hacks(raw);
        assert_eq!(dto.hacks.len(), 1);
        assert_eq!(dto.hacks[0].name, "Hack1");
        assert_eq!(dto.hacks[0].date, Some(1672531200));
    }

    #[test]
    fn map_institutions_joins_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "1".to_string(),
            client::RawInstitutionMeta {
                ticker: Some("MSTR".into()),
                name: Some("MicroStrategy".into()),
                inst_type: Some("Company".into()),
                total_usd_value: None,
                total_cost: None,
            },
        );
        let raw = client::RawInstitutionsResponse {
            institution_metadata: metadata,
            institutions: vec![
                client::RawInstitutionEntry {
                    institution_id: Some(1),
                    total_usd_value: Some(10_000_000_000.0),
                    total_cost: Some(5_000_000_000.0),
                },
                client::RawInstitutionEntry {
                    institution_id: None,
                    total_usd_value: None,
                    total_cost: None,
                },
            ],
        };
        let dto = map_institutions(raw);
        assert_eq!(dto.institutions.len(), 1);
        assert_eq!(dto.institutions[0].name, "MicroStrategy");
        assert_eq!(dto.institutions[0].ticker, Some("MSTR".to_owned()));
        assert_eq!(dto.institutions[0].total_value_usd, Some(10_000_000_000.0));
    }

    #[test]
    fn map_fdv_performance_time_series() {
        let raw = vec![serde_json::json!({
            "date": 1700000000,
            "Bitcoin": 0.052,
            "DeFi": -0.031
        })];
        let dto = map_fdv_performance(raw, "30");
        assert_eq!(dto.period, "30");
        assert_eq!(dto.points.len(), 1);
        assert_eq!(dto.points[0].date, Some(1700000000));
        assert_eq!(dto.points[0].categories.len(), 2);
    }

    #[test]
    fn map_usage_basic() {
        let raw = client::RawUsage {
            requests_today: Some(100),
            requests_this_month: Some(5000),
            rate_limit: Some(10000),
        };
        let dto = map_usage(raw);
        assert_eq!(dto.requests_today, Some(100));
    }
}
