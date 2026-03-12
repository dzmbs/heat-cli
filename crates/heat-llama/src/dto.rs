/// Heat-owned DTOs for DefiLlama protocol output.
///
/// These types are the stable public contract for `heat llama` commands.
/// Raw DefiLlama API responses are mapped into these types in `map.rs`.
use serde::Serialize;

// ---------------------------------------------------------------------------
// Protocols
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ProtocolRow {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub symbol: Option<String>,
    pub category: Option<String>,
    pub chains: Vec<String>,
    pub tvl_usd: Option<f64>,
    pub change_1d_pct: Option<f64>,
    pub change_7d_pct: Option<f64>,
    pub change_1m_pct: Option<f64>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProtocolsListDto {
    pub protocols: Vec<ProtocolRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProtocolDetailDto {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub symbol: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub chains: Vec<String>,
    pub tvl_usd: Option<f64>,
    pub chain_tvls: std::collections::HashMap<String, f64>,
    pub mcap_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProtocolTvlDto {
    pub slug: String,
    pub tvl_usd: f64,
}

// ---------------------------------------------------------------------------
// Chains
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ChainRow {
    pub name: String,
    pub token_symbol: Option<String>,
    pub tvl_usd: Option<f64>,
    pub chain_id: Option<u64>,
    pub gecko_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChainsListDto {
    pub chains: Vec<ChainRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TvlPoint {
    pub date: i64,
    pub tvl_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChainHistoryDto {
    pub chain: Option<String>,
    pub points: Vec<TvlPoint>,
}

// ---------------------------------------------------------------------------
// Coins
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CoinPrice {
    pub coin: String,
    pub price_usd: Option<f64>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub timestamp: Option<i64>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoinsPriceDto {
    pub prices: Vec<CoinPrice>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChartPoint {
    pub timestamp: i64,
    pub price_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoinChartEntry {
    pub coin: String,
    pub symbol: Option<String>,
    pub points: Vec<ChartPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoinsChartDto {
    pub coins: Vec<CoinChartEntry>,
}

// ---------------------------------------------------------------------------
// Coins change / first / block / liquidity / batch-historical
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CoinChangeEntry {
    pub coin: String,
    pub change_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoinsChangeDto {
    pub period: Option<String>,
    pub coins: Vec<CoinChangeEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockDto {
    pub chain: String,
    pub height: Option<u64>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiquidityPoint {
    pub date: i64,
    pub liquidity_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoinLiquidityDto {
    pub token: String,
    pub points: Vec<LiquidityPoint>,
}

// ---------------------------------------------------------------------------
// Stablecoins
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinRow {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub peg_type: Option<String>,
    pub peg_mechanism: Option<String>,
    pub price: Option<f64>,
    pub circulating_usd: Option<f64>,
    pub chains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinsListDto {
    pub stablecoins: Vec<StablecoinRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinDetailDto {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub peg_type: Option<String>,
    pub peg_mechanism: Option<String>,
    pub price: Option<f64>,
    pub chains: Vec<String>,
    pub chain_circulating: std::collections::HashMap<String, f64>,
}

// ---------------------------------------------------------------------------
// Stablecoin chains / charts / dominance / prices
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinChainRow {
    pub name: String,
    pub gecko_id: Option<String>,
    pub circulating_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinChainsDto {
    pub chains: Vec<StablecoinChainRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinChartPoint {
    pub date: i64,
    pub circulating_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinChartDto {
    pub chain: Option<String>,
    pub points: Vec<StablecoinChartPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinDominanceEntry {
    pub name: String,
    pub dominance_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinDominancePoint {
    pub date: i64,
    pub total_circulating_usd: Option<f64>,
    pub dominance: Vec<StablecoinDominanceEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinDominanceDto {
    pub chain: String,
    pub points: Vec<StablecoinDominancePoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinPriceEntry {
    pub name: String,
    pub price: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinPricesPoint {
    pub date: i64,
    pub prices: Vec<StablecoinPriceEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StablecoinPricesDto {
    pub points: Vec<StablecoinPricesPoint>,
}

// ---------------------------------------------------------------------------
// Bridges
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct BridgeRow {
    pub id: u64,
    pub name: String,
    pub daily_volume_usd: Option<f64>,
    pub weekly_volume_usd: Option<f64>,
    pub monthly_volume_usd: Option<f64>,
    pub chains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgesListDto {
    pub bridges: Vec<BridgeRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeDetailDto {
    pub id: u64,
    pub name: String,
    pub chains: Vec<String>,
    pub destination_chain: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeVolumePoint {
    pub date: i64,
    pub deposit_usd: Option<f64>,
    pub withdraw_usd: Option<f64>,
    pub deposit_txs: Option<u64>,
    pub withdraw_txs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeVolumeDto {
    pub chain: String,
    pub points: Vec<BridgeVolumePoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeDayStatEntry {
    pub date: i64,
    pub tokens_deposited_count: usize,
    pub tokens_withdrawn_count: usize,
    pub addresses_deposited_count: usize,
    pub addresses_withdrawn_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeDayStatsDto {
    pub chain: String,
    pub timestamp: i64,
    pub stats: Vec<BridgeDayStatEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeTxRow {
    pub tx_hash: String,
    pub timestamp: Option<i64>,
    pub chain: Option<String>,
    pub token: Option<String>,
    pub amount: Option<String>,
    pub is_deposit: Option<bool>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeTxDto {
    pub bridge_id: u64,
    pub transactions: Vec<BridgeTxRow>,
}

// ---------------------------------------------------------------------------
// Yields
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct YieldPoolRow {
    pub pool: String,
    pub chain: Option<String>,
    pub project: Option<String>,
    pub symbol: Option<String>,
    pub tvl_usd: Option<f64>,
    pub apy: Option<f64>,
    pub apy_base: Option<f64>,
    pub apy_reward: Option<f64>,
    pub stablecoin: Option<bool>,
    pub il_risk: Option<String>,
    pub exposure: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YieldPoolsDto {
    pub pools: Vec<YieldPoolRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YieldBorrowPoolRow {
    pub pool: String,
    pub chain: Option<String>,
    pub project: Option<String>,
    pub symbol: Option<String>,
    pub tvl_usd: Option<f64>,
    pub apy: Option<f64>,
    pub apy_base: Option<f64>,
    pub apy_reward: Option<f64>,
    pub apy_base_borrow: Option<f64>,
    pub apy_reward_borrow: Option<f64>,
    pub total_supply_usd: Option<f64>,
    pub total_borrow_usd: Option<f64>,
    pub stablecoin: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YieldBorrowPoolsDto {
    pub pools: Vec<YieldBorrowPoolRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YieldChartPoint {
    pub timestamp: String,
    pub tvl_usd: Option<f64>,
    pub apy: Option<f64>,
    pub apy_base: Option<f64>,
    pub apy_reward: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YieldChartDto {
    pub pool: String,
    pub points: Vec<YieldChartPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YieldLendBorrowChartPoint {
    pub timestamp: String,
    pub tvl_usd: Option<f64>,
    pub apy: Option<f64>,
    pub apy_base: Option<f64>,
    pub apy_reward: Option<f64>,
    pub apy_base_borrow: Option<f64>,
    pub apy_reward_borrow: Option<f64>,
    pub total_supply_usd: Option<f64>,
    pub total_borrow_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YieldLendBorrowChartDto {
    pub pool: String,
    pub points: Vec<YieldLendBorrowChartPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerpRow {
    pub marketplace: Option<String>,
    pub symbol: Option<String>,
    pub base_asset: Option<String>,
    pub funding_rate: Option<f64>,
    pub open_interest: Option<f64>,
    pub index_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerpsDto {
    pub perps: Vec<PerpRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LsdRow {
    pub name: String,
    pub symbol: Option<String>,
    pub eth_peg: Option<f64>,
    pub apy: Option<f64>,
    pub market_share: Option<f64>,
    pub fee: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LsdDto {
    pub rates: Vec<LsdRow>,
}

// ---------------------------------------------------------------------------
// Ecosystem / Intelligence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CategoryRow {
    pub name: String,
    pub protocol_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoriesDto {
    pub categories: Vec<CategoryRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForkRow {
    pub name: String,
    pub tvl_usd: Option<f64>,
    pub fork_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForksDto {
    pub forks: Vec<ForkRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OracleRow {
    pub name: String,
    pub tvl_secured_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OraclesDto {
    pub oracles: Vec<OracleRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityRow {
    pub name: String,
    pub category: Option<String>,
    pub tvl_usd: Option<f64>,
    pub change_1d_pct: Option<f64>,
    pub change_7d_pct: Option<f64>,
    pub chains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntitiesDto {
    pub entities: Vec<EntityRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaiseRow {
    pub name: String,
    pub round: Option<String>,
    pub amount_usd: Option<f64>,
    pub date: Option<i64>,
    pub lead_investors: Vec<String>,
    pub category: Option<String>,
    pub chains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RaisesDto {
    pub raises: Vec<RaiseRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TreasuryRow {
    pub name: String,
    pub symbol: Option<String>,
    pub category: Option<String>,
    pub tvl_usd: Option<f64>,
    pub change_1d_pct: Option<f64>,
    pub change_7d_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TreasuriesDto {
    pub treasuries: Vec<TreasuryRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HackRow {
    pub name: String,
    pub date: Option<i64>,
    pub amount_usd: Option<f64>,
    pub chains: Vec<String>,
    pub classification: Option<String>,
    pub technique: Option<String>,
    pub target_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HacksDto {
    pub hacks: Vec<HackRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InflowsDto {
    pub protocol: String,
    pub outflows_usd: Option<f64>,
    pub old_tokens_date: Option<i64>,
    pub current_tokens_date: Option<i64>,
    pub old_tokens: std::collections::HashMap<String, f64>,
    pub current_tokens: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenProtocolRow {
    pub name: String,
    pub category: Option<String>,
    pub total_amount_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenProtocolsDto {
    pub symbol: String,
    pub protocols: Vec<TokenProtocolRow>,
}

// ---------------------------------------------------------------------------
// Fees / Volumes overview
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ProtocolSummaryDto {
    pub metric: String,
    pub name: String,
    pub slug: Option<String>,
    pub category: Option<String>,
    pub chains: Vec<String>,
    pub total_24h_usd: Option<f64>,
    pub total_7d_usd: Option<f64>,
    pub total_30d_usd: Option<f64>,
    pub change_1d_pct: Option<f64>,
    pub change_7d_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OverviewProtocolRow {
    pub name: String,
    pub slug: Option<String>,
    pub category: Option<String>,
    pub total_24h_usd: Option<f64>,
    pub total_7d_usd: Option<f64>,
    pub change_1d_pct: Option<f64>,
    pub change_7d_pct: Option<f64>,
    pub chains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OverviewDto {
    pub metric: String,
    pub chain: Option<String>,
    pub total_24h_usd: Option<f64>,
    pub total_7d_usd: Option<f64>,
    pub change_1d_pct: Option<f64>,
    pub change_7d_pct: Option<f64>,
    pub protocols: Vec<OverviewProtocolRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricHistoryPoint {
    pub timestamp: i64,
    pub value_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricHistoryDto {
    pub metric: String,
    pub target: String,
    pub data_type: String,
    pub points: Vec<MetricHistoryPoint>,
}

// ---------------------------------------------------------------------------
// Institutions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct InstitutionRow {
    pub name: String,
    pub ticker: Option<String>,
    pub inst_type: Option<String>,
    pub total_value_usd: Option<f64>,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstitutionsDto {
    pub institutions: Vec<InstitutionRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstitutionDetailDto {
    pub name: String,
    pub ticker: Option<String>,
    pub inst_type: Option<String>,
    pub price: Option<f64>,
    pub total_value_usd: Option<f64>,
    pub total_cost_usd: Option<f64>,
}

// ---------------------------------------------------------------------------
// ETFs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct EtfSnapshotRow {
    pub ticker: Option<String>,
    pub name: Option<String>,
    pub issuer: Option<String>,
    pub asset: Option<String>,
    pub fee_pct: Option<f64>,
    pub flows_usd: Option<f64>,
    pub aum_usd: Option<f64>,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EtfSnapshotDto {
    pub etfs: Vec<EtfSnapshotRow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EtfFlowPoint {
    pub date: String,
    pub gecko_id: Option<String>,
    pub total_flow_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EtfFlowsDto {
    pub flows: Vec<EtfFlowPoint>,
}

// ---------------------------------------------------------------------------
// FDV
// ---------------------------------------------------------------------------

/// FDV performance is a time-series of category performance values.
#[derive(Debug, Clone, Serialize)]
pub struct FdvCategoryEntry {
    pub category: String,
    pub performance: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FdvPerformancePoint {
    pub date: Option<i64>,
    pub categories: Vec<FdvCategoryEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FdvPerformanceDto {
    pub period: String,
    pub points: Vec<FdvPerformancePoint>,
}

// ---------------------------------------------------------------------------
// Usage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct UsageDto {
    pub requests_today: Option<u64>,
    pub requests_this_month: Option<u64>,
    pub rate_limit: Option<u64>,
}
