/// DefiLlama REST API client.
///
/// Supports multiple API families (TVL, coins, stablecoins, etc.) each with
/// their own free base URL. Pro API key is path-based on a unified domain.
use heat_core::error::HeatError;
use serde::Deserialize;

const PRO_BASE_URL: &str = "https://pro-api.llama.fi";

/// Deserialize a timestamp that may be either a unix integer or an ISO 8601 string.
fn deserialize_flexible_ts<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct FlexibleTs;

    impl<'de> de::Visitor<'de> for FlexibleTs {
        type Value = Option<i64>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a unix timestamp (integer) or ISO 8601 string")
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as i64))
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            Ok(Some(v as i64))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            // Try parsing ISO 8601 → unix seconds (simplified: just handle the common format)
            // "2026-03-10T23:16:23.000Z"
            parse_iso_to_unix(v).map(Some).map_err(de::Error::custom)
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }

    deserializer.deserialize_any(FlexibleTs)
}

/// Parse a subset of ISO 8601 into unix seconds.
fn parse_iso_to_unix(s: &str) -> Result<i64, String> {
    // Expected: "YYYY-MM-DDTHH:MM:SS.sssZ" or "YYYY-MM-DDTHH:MM:SSZ"
    let s = s.trim_end_matches('Z');
    let (date_part, time_part) = s
        .split_once('T')
        .ok_or_else(|| format!("not ISO 8601: {s}"))?;
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("bad date: {date_part}"));
    }
    let y: i64 = parts[0].parse().map_err(|e| format!("{e}"))?;
    let m: u32 = parts[1].parse().map_err(|e| format!("{e}"))?;
    let d: u32 = parts[2].parse().map_err(|e| format!("{e}"))?;

    let time_no_frac = time_part.split('.').next().unwrap_or(time_part);
    let tp: Vec<&str> = time_no_frac.split(':').collect();
    if tp.len() != 3 {
        return Err(format!("bad time: {time_part}"));
    }
    let h: i64 = tp[0].parse().map_err(|e| format!("{e}"))?;
    let min: i64 = tp[1].parse().map_err(|e| format!("{e}"))?;
    let sec: i64 = tp[2].parse().map_err(|e| format!("{e}"))?;

    // Days from civil date (Howard Hinnant algorithm)
    let m_adj = if m <= 2 { m + 9 } else { m - 3 };
    let y_adj = if m <= 2 { y - 1 } else { y };
    let era = if y_adj >= 0 { y_adj } else { y_adj - 399 } / 400;
    let yoe = (y_adj - era * 400) as u32;
    let doy = (153 * m_adj + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe as i64 - 719468;

    Ok(days * 86400 + h * 3600 + min * 60 + sec)
}

// ---------------------------------------------------------------------------
// API family — determines base URL routing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum ApiFamily {
    /// TVL, protocols, chains, volumes, fees — api.llama.fi
    Main,
    /// Coin prices and history — coins.llama.fi
    Coins,
    /// Stablecoin data — stablecoins.llama.fi
    Stablecoins,
    /// Yield/APY data — yields.llama.fi
    Yields,
    /// Bridge data — bridges.llama.fi
    Bridges,
}

impl ApiFamily {
    fn free_base_url(self) -> &'static str {
        match self {
            Self::Main => "https://api.llama.fi",
            Self::Coins => "https://coins.llama.fi",
            Self::Stablecoins => "https://stablecoins.llama.fi",
            Self::Yields => "https://yields.llama.fi",
            Self::Bridges => "https://bridges.llama.fi",
        }
    }
}

// ---------------------------------------------------------------------------
// Raw API response types (internal — not part of Heat's output contract)
// ---------------------------------------------------------------------------

// --- TVL / protocols ---

#[derive(Debug, Deserialize)]
pub struct RawProtocol {
    pub id: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub slug: Option<String>,
    pub category: Option<String>,
    #[serde(default)]
    pub chains: Vec<String>,
    pub tvl: Option<f64>,
    pub change_1d: Option<f64>,
    pub change_7d: Option<f64>,
    pub change_1m: Option<f64>,
    pub url: Option<String>,
    pub logo: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawProtocolDetail {
    pub id: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub slug: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub logo: Option<String>,
    #[serde(default)]
    pub chains: Vec<String>,
    pub tvl: Option<serde_json::Value>,
    #[serde(default)]
    pub current_chain_tvls: std::collections::HashMap<String, f64>,
    pub mcap: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct RawTvlPoint {
    pub date: Option<i64>,
    pub tvl: Option<f64>,
    #[serde(rename = "totalLiquidityUSD")]
    pub total_liquidity_usd: Option<f64>,
}

// --- Chains ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawChain {
    pub name: Option<String>,
    pub gecko_id: Option<String>,
    pub token_symbol: Option<String>,
    pub tvl: Option<f64>,
    /// Can be number or string in upstream data.
    pub chain_id: Option<serde_json::Value>,
    pub cmc_id: Option<String>,
}

// --- Coins ---

#[derive(Debug, Deserialize)]
pub struct RawCoinsResponse {
    pub coins: std::collections::HashMap<String, RawCoinPrice>,
}

// coins change response — just a map of coin -> percentage
#[derive(Debug, Deserialize)]
pub struct RawCoinsChangeResponse {
    pub coins: std::collections::HashMap<String, f64>,
}

// block response
#[derive(Debug, Deserialize)]
pub struct RawBlockResponse {
    pub height: Option<u64>,
    pub timestamp: Option<i64>,
}

// liquidity point
#[derive(Debug, Deserialize)]
pub struct RawLiquidityPoint {
    pub date: Option<i64>,
    pub liquidity: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct RawCoinPrice {
    pub price: Option<f64>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub timestamp: Option<i64>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct RawCoinChart {
    pub coins: std::collections::HashMap<String, RawCoinChartData>,
}

#[derive(Debug, Deserialize)]
pub struct RawCoinChartData {
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub confidence: Option<f64>,
    #[serde(default)]
    pub prices: Vec<RawPricePoint>,
}

#[derive(Debug, Deserialize)]
pub struct RawPricePoint {
    pub timestamp: Option<i64>,
    pub price: Option<f64>,
}

// --- Stablecoins ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawStablecoinsResponse {
    #[serde(default)]
    pub pegged_assets: Vec<RawStablecoin>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawStablecoin {
    pub id: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub gecko_id: Option<String>,
    pub peg_type: Option<String>,
    pub peg_mechanism: Option<String>,
    pub price: Option<f64>,
    pub circulating: Option<serde_json::Value>,
    #[serde(default)]
    pub chains: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawStablecoinDetail {
    pub id: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub gecko_id: Option<String>,
    pub peg_type: Option<String>,
    pub peg_mechanism: Option<String>,
    pub price: Option<f64>,
    pub price_source: Option<String>,
    /// Upstream uses currentChainBalances: { "Ethereum": { "peggedUSD": N }, ... }
    pub current_chain_balances: Option<serde_json::Value>,
}

// --- Stablecoin chains / charts / dominance / prices ---

#[derive(Debug, Deserialize)]
pub struct RawStablecoinChain {
    pub gecko_id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "totalCirculatingUSD")]
    pub total_circulating_usd: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawStablecoinChartPoint {
    pub date: Option<serde_json::Value>,
    #[serde(rename = "totalCirculating")]
    pub total_circulating: Option<serde_json::Value>,
    #[serde(rename = "totalCirculatingUSD")]
    pub total_circulating_usd: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawStablecoinDominancePoint {
    pub date: Option<i64>,
    #[serde(rename = "totalCirculating")]
    pub total_circulating: Option<serde_json::Value>,
    pub dominance: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawStablecoinPricePoint {
    pub date: Option<i64>,
    pub prices: Option<serde_json::Value>,
}

// --- Bridges ---

#[derive(Debug, Deserialize)]
pub struct RawBridgesResponse {
    #[serde(default)]
    pub bridges: Vec<RawBridge>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawBridge {
    pub id: Option<u64>,
    pub display_name: Option<String>,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub last_hourly_volume: Option<f64>,
    pub current_day_volume: Option<f64>,
    pub last_daily_volume: Option<f64>,
    pub weekly_volume: Option<f64>,
    pub monthly_volume: Option<f64>,
    #[serde(default)]
    pub chains: Vec<String>,
    pub destination_chain: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawBridgeDetail {
    pub id: Option<u64>,
    pub display_name: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub chains: Vec<String>,
    pub destination_chain: Option<String>,
    #[serde(default)]
    pub chain_breakdown: serde_json::Value,
}

// --- Bridge volume / day stats / transactions ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawBridgeVolumePoint {
    pub date: Option<i64>,
    #[serde(rename = "depositUSD")]
    pub deposit_usd: Option<f64>,
    #[serde(rename = "withdrawUSD")]
    pub withdraw_usd: Option<f64>,
    pub deposit_txs: Option<u64>,
    pub withdraw_txs: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawBridgeDayStats {
    pub date: Option<i64>,
    pub total_tokens_deposited: Option<serde_json::Value>,
    pub total_tokens_withdrawn: Option<serde_json::Value>,
    pub total_address_deposited: Option<serde_json::Value>,
    pub total_address_withdrawn: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RawBridgeTx {
    pub tx_hash: Option<String>,
    /// Upstream sends either a unix timestamp (i64) or an ISO string.
    #[serde(default, deserialize_with = "deserialize_flexible_ts")]
    pub ts: Option<i64>,
    pub tx_block: Option<u64>,
    pub tx_from: Option<String>,
    pub tx_to: Option<String>,
    pub token: Option<String>,
    pub amount: Option<String>,
    pub is_deposit: Option<bool>,
    pub chain: Option<String>,
}

// --- Fees / Volumes ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSummaryResponse {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub category: Option<String>,
    #[serde(default)]
    pub chains: Vec<String>,
    pub total24h: Option<f64>,
    pub total48hto24h: Option<f64>,
    pub total7d: Option<f64>,
    pub total30d: Option<f64>,
    pub total1y: Option<f64>,
    pub change_1d: Option<f64>,
    pub change_7d: Option<f64>,
    pub change_1m: Option<f64>,
    pub chain_data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawOverviewResponse {
    pub total24h: Option<f64>,
    pub total48hto24h: Option<f64>,
    pub total7d: Option<f64>,
    pub total30d: Option<f64>,
    pub total1y: Option<f64>,
    pub change_1d: Option<f64>,
    pub change_7d: Option<f64>,
    pub change_1m: Option<f64>,
    #[serde(default)]
    pub protocols: Vec<RawOverviewProtocol>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawOverviewProtocol {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub category: Option<String>,
    #[serde(default)]
    pub chains: Vec<String>,
    pub total24h: Option<f64>,
    pub total48hto24h: Option<f64>,
    pub total7d: Option<f64>,
    pub total30d: Option<f64>,
    pub change_1d: Option<f64>,
    pub change_7d: Option<f64>,
    pub change_1m: Option<f64>,
}

// --- Yields ---

#[derive(Debug, Deserialize)]
pub struct RawYieldsResponse<T> {
    pub status: Option<String>,
    #[serde(default)]
    pub data: Vec<T>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawYieldPool {
    pub chain: Option<String>,
    pub project: Option<String>,
    pub symbol: Option<String>,
    pub pool: Option<String>,
    pub tvl_usd: Option<f64>,
    pub apy: Option<f64>,
    pub apy_base: Option<f64>,
    pub apy_reward: Option<f64>,
    pub stablecoin: Option<bool>,
    pub il_risk: Option<String>,
    pub exposure: Option<String>,
    pub pool_meta: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawYieldBorrowPool {
    pub chain: Option<String>,
    pub project: Option<String>,
    pub symbol: Option<String>,
    pub pool: Option<String>,
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

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawYieldChartPoint {
    pub timestamp: Option<String>,
    pub tvl_usd: Option<f64>,
    pub apy: Option<f64>,
    pub apy_base: Option<f64>,
    pub apy_reward: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawYieldLendBorrowChartPoint {
    pub timestamp: Option<String>,
    pub tvl_usd: Option<f64>,
    pub apy: Option<f64>,
    pub apy_base: Option<f64>,
    pub apy_reward: Option<f64>,
    pub apy_base_borrow: Option<f64>,
    pub apy_reward_borrow: Option<f64>,
    pub total_supply_usd: Option<f64>,
    pub total_borrow_usd: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawPerp {
    pub marketplace: Option<String>,
    pub symbol: Option<String>,
    pub funding_rate: Option<f64>,
    pub open_interest: Option<f64>,
    pub index_price: Option<f64>,
    pub base_asset: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawLsdRate {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub eth_peg: Option<f64>,
    pub apy: Option<f64>,
    pub market_share: Option<f64>,
    pub fee: Option<f64>,
}

// --- Ecosystem ---

/// `/api/categories` returns `{chart: {...}, categories: {name: [protocols...]}}`.
#[derive(Debug, Deserialize)]
pub struct RawCategoriesResponse {
    #[serde(default)]
    pub categories: std::collections::HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct RawForksResponse {
    #[serde(default)]
    pub forks: std::collections::HashMap<String, RawForkEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawForkEntry {
    #[serde(default)]
    pub forked_protocols: Vec<String>,
    pub tvl: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct RawOraclesResponse {
    #[serde(default)]
    pub oracles: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawEntity {
    pub name: Option<String>,
    pub category: Option<String>,
    pub slug: Option<String>,
    pub tvl: Option<f64>,
    pub change_1d: Option<f64>,
    pub change_7d: Option<f64>,
    #[serde(default)]
    pub chains: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawRaisesResponse {
    #[serde(default)]
    pub raises: Vec<RawRaise>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawRaise {
    pub name: Option<String>,
    pub round: Option<String>,
    pub amount: Option<f64>,
    pub date: Option<i64>,
    #[serde(default)]
    pub lead_investors: Vec<String>,
    pub category: Option<String>,
    #[serde(default)]
    pub chains: Vec<String>,
}

/// `/api/treasuries` returns array of protocol-like objects.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTreasury {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub category: Option<String>,
    pub tvl: Option<f64>,
    pub change_1d: Option<f64>,
    pub change_7d: Option<f64>,
}

/// `/api/hacks` returns array. `date` is unix timestamp (number), `chain` is array of strings.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawHack {
    pub name: Option<String>,
    pub date: Option<i64>,
    pub amount: Option<f64>,
    #[serde(default)]
    pub chain: Vec<String>,
    pub classification: Option<String>,
    pub technique: Option<String>,
    pub target_type: Option<String>,
    pub bridge_hack: Option<bool>,
}

/// `/api/inflows/{protocol}/{timestamp}` returns `{outflows, oldTokens, currentTokens}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawInflowsResponse {
    pub outflows: Option<f64>,
    pub old_tokens: Option<RawInflowTokens>,
    pub current_tokens: Option<RawInflowTokens>,
}

#[derive(Debug, Deserialize)]
pub struct RawInflowTokens {
    pub date: Option<i64>,
    #[serde(default)]
    pub tvl: std::collections::HashMap<String, f64>,
}

/// `/api/tokenProtocols/{symbol}` returns array of `{name, category, amountUsd}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTokenProtocol {
    pub name: Option<String>,
    pub category: Option<String>,
    #[serde(default)]
    pub amount_usd: std::collections::HashMap<String, f64>,
}

// --- Institutions ---

/// `/dat/institutions` returns `{institutionMetadata, assetMetadata, institutions, assets}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawInstitutionsResponse {
    #[serde(default)]
    pub institution_metadata: std::collections::HashMap<String, RawInstitutionMeta>,
    #[serde(default)]
    pub institutions: Vec<RawInstitutionEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawInstitutionMeta {
    pub ticker: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub inst_type: Option<String>,
    pub total_usd_value: Option<f64>,
    pub total_cost: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawInstitutionEntry {
    pub institution_id: Option<u64>,
    pub total_usd_value: Option<f64>,
    pub total_cost: Option<f64>,
}

/// `/dat/institutions/{symbol}` returns a single institution detail object.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawInstitutionDetail {
    pub institution_id: Option<u64>,
    pub ticker: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub inst_type: Option<String>,
    pub price: Option<f64>,
    pub total_usd_value: Option<f64>,
    pub total_cost: Option<f64>,
}

// --- ETFs ---

/// `/etfs/snapshot` — field names from OpenAPI spec.
#[derive(Debug, Deserialize)]
pub struct RawEtfSnapshot {
    pub ticker: Option<String>,
    pub etf_name: Option<String>,
    pub issuer: Option<String>,
    pub asset: Option<String>,
    pub pct_fee: Option<f64>,
    pub flows: Option<f64>,
    pub aum: Option<f64>,
    pub volume: Option<f64>,
}

/// `/etfs/flows` — `day` is ISO date string, `total_flow_usd` is the flow amount.
#[derive(Debug, Deserialize)]
pub struct RawEtfFlow {
    pub gecko_id: Option<String>,
    pub day: Option<String>,
    pub total_flow_usd: Option<f64>,
}

// --- FDV ---

/// `/fdv/performance/{period}` returns time-series with dynamic category performance fields.
/// Each entry has a `date` and then dynamic category names → performance numbers.
/// We deserialize as generic JSON and extract in the mapper.

// --- Usage ---

#[derive(Debug, Deserialize)]
pub struct RawUsage {
    pub requests_today: Option<u64>,
    pub requests_this_month: Option<u64>,
    pub rate_limit: Option<u64>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct DefiLlamaClient {
    client: reqwest::Client,
    api_key: Option<String>,
    /// Override base URL for testing.
    #[allow(dead_code)]
    base_override: Option<String>,
}

impl DefiLlamaClient {
    pub fn new(api_key: Option<String>) -> Result<Self, HeatError> {
        let client = reqwest::Client::builder()
            .user_agent("heat-cli")
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                HeatError::internal("http_client", format!("Failed to build HTTP client: {e}"))
            })?;
        Ok(Self {
            client,
            api_key,
            base_override: None,
        })
    }

    #[cfg(test)]
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self, HeatError> {
        let client = reqwest::Client::builder()
            .user_agent("heat-cli")
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                HeatError::internal("http_client", format!("Failed to build HTTP client: {e}"))
            })?;
        Ok(Self {
            client,
            api_key: None,
            base_override: Some(base_url.into()),
        })
    }

    /// Build the full URL for a request.
    ///
    /// - Free: `{family_base_url}/{path}`
    /// - Pro:  `https://pro-api.llama.fi/{api_key}/{path}`
    pub fn url(&self, family: ApiFamily, path: &str) -> String {
        let path = path.trim_start_matches('/');

        if let Some(base) = &self.base_override {
            return format!("{}/{path}", base.trim_end_matches('/'));
        }

        if let Some(key) = &self.api_key {
            return format!("{PRO_BASE_URL}/{key}/{path}");
        }

        format!("{}/{path}", family.free_base_url())
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        family: ApiFamily,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, HeatError> {
        let url = self.url(family, path);
        let resp = self
            .client
            .get(&url)
            .query(query)
            .send()
            .await
            .map_err(|e| {
                HeatError::network("request_failed", format!("DefiLlama request failed: {e}"))
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let msg = format!("DefiLlama API returned {status}: {body}");
            return Err(if status.as_u16() == 401 || status.as_u16() == 403 {
                HeatError::auth("api_error", msg)
                    .with_hint("This endpoint may require a Pro API key. Set HEAT_DEFILLAMA_API_KEY or add api_key under [protocols.defillama]")
            } else if status.as_u16() == 429 || status.is_server_error() {
                HeatError::network("api_error", msg)
            } else {
                HeatError::protocol("api_error", msg)
            });
        }

        let text = resp.text().await.map_err(|e| {
            HeatError::protocol(
                "parse_error",
                format!("Failed to read DefiLlama response: {e}"),
            )
        })?;
        serde_json::from_str::<T>(&text).map_err(|e| {
            HeatError::protocol(
                "parse_error",
                format!("Failed to parse DefiLlama response: {e}"),
            )
        })
    }

    /// Raw text GET (for endpoints that return a bare number like /tvl/{slug}).
    async fn get_text(&self, family: ApiFamily, path: &str) -> Result<String, HeatError> {
        let url = self.url(family, path);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            HeatError::network("request_failed", format!("DefiLlama request failed: {e}"))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(HeatError::protocol(
                "api_error",
                format!("DefiLlama API returned {status}: {body}"),
            ));
        }

        resp.text().await.map_err(|e| {
            HeatError::protocol(
                "parse_error",
                format!("Failed to read DefiLlama response: {e}"),
            )
        })
    }

    // -----------------------------------------------------------------------
    // TVL / Protocols
    // -----------------------------------------------------------------------

    pub async fn protocols(&self) -> Result<Vec<RawProtocol>, HeatError> {
        self.get_json(ApiFamily::Main, "protocols", &[]).await
    }

    pub async fn protocol(&self, slug: &str) -> Result<RawProtocolDetail, HeatError> {
        self.get_json(ApiFamily::Main, &format!("protocol/{slug}"), &[])
            .await
    }

    pub async fn tvl(&self, slug: &str) -> Result<f64, HeatError> {
        let text = self
            .get_text(ApiFamily::Main, &format!("tvl/{slug}"))
            .await?;
        text.trim().parse::<f64>().map_err(|_| {
            HeatError::protocol("parse_error", format!("Expected numeric TVL, got: {text}"))
        })
    }

    // -----------------------------------------------------------------------
    // Chains
    // -----------------------------------------------------------------------

    pub async fn chains(&self) -> Result<Vec<RawChain>, HeatError> {
        self.get_json(ApiFamily::Main, "v2/chains", &[]).await
    }

    pub async fn historical_chain_tvl(
        &self,
        chain: Option<&str>,
    ) -> Result<Vec<RawTvlPoint>, HeatError> {
        let path = match chain {
            Some(c) => format!("v2/historicalChainTvl/{c}"),
            None => "v2/historicalChainTvl".to_owned(),
        };
        self.get_json(ApiFamily::Main, &path, &[]).await
    }

    // -----------------------------------------------------------------------
    // Coins
    // -----------------------------------------------------------------------

    pub async fn coins_price(&self, coins: &str) -> Result<RawCoinsResponse, HeatError> {
        self.get_json(ApiFamily::Coins, &format!("prices/current/{coins}"), &[])
            .await
    }

    pub async fn coins_historical(
        &self,
        timestamp: i64,
        coins: &str,
    ) -> Result<RawCoinsResponse, HeatError> {
        self.get_json(
            ApiFamily::Coins,
            &format!("prices/historical/{timestamp}/{coins}"),
            &[],
        )
        .await
    }

    pub async fn coins_chart(
        &self,
        coins: &str,
        period: Option<&str>,
        span: Option<u32>,
    ) -> Result<RawCoinChart, HeatError> {
        let mut query = Vec::new();
        let period_str;
        if let Some(p) = period {
            period_str = p.to_owned();
            query.push(("period", period_str.as_str()));
        }
        let span_str;
        if let Some(s) = span {
            span_str = s.to_string();
            query.push(("span", span_str.as_str()));
        }
        self.get_json(ApiFamily::Coins, &format!("chart/{coins}"), &query)
            .await
    }

    pub async fn coins_change(
        &self,
        coins: &str,
        period: Option<&str>,
    ) -> Result<RawCoinsChangeResponse, HeatError> {
        let mut query = Vec::new();
        let period_str;
        if let Some(p) = period {
            period_str = p.to_owned();
            query.push(("period", period_str.as_str()));
        }
        self.get_json(ApiFamily::Coins, &format!("percentage/{coins}"), &query)
            .await
    }

    pub async fn coins_first(&self, coins: &str) -> Result<RawCoinsResponse, HeatError> {
        self.get_json(ApiFamily::Coins, &format!("prices/first/{coins}"), &[])
            .await
    }

    pub async fn coins_block(
        &self,
        chain: &str,
        timestamp: i64,
    ) -> Result<RawBlockResponse, HeatError> {
        self.get_json(ApiFamily::Coins, &format!("block/{chain}/{timestamp}"), &[])
            .await
    }

    pub async fn coins_liquidity(&self, token: &str) -> Result<Vec<RawLiquidityPoint>, HeatError> {
        self.get_json(
            ApiFamily::Coins,
            &format!("api/historicalLiquidity/{token}"),
            &[],
        )
        .await
    }

    async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        family: ApiFamily,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<T, HeatError> {
        let url = self.url(family, path);
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| {
                HeatError::network("request_failed", format!("DefiLlama request failed: {e}"))
            })?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let msg = format!("DefiLlama API returned {status}: {body}");
            return Err(if status.as_u16() == 401 || status.as_u16() == 403 {
                HeatError::auth("api_error", msg)
                    .with_hint("This endpoint may require a Pro API key")
            } else {
                HeatError::protocol("api_error", msg)
            });
        }
        let text = resp.text().await.map_err(|e| {
            HeatError::protocol(
                "parse_error",
                format!("Failed to read DefiLlama response: {e}"),
            )
        })?;
        serde_json::from_str::<T>(&text).map_err(|e| {
            HeatError::protocol(
                "parse_error",
                format!("Failed to parse DefiLlama response: {e}"),
            )
        })
    }

    pub async fn coins_batch_historical(
        &self,
        body: &serde_json::Value,
    ) -> Result<RawCoinsResponse, HeatError> {
        self.post_json(ApiFamily::Coins, "batchHistorical", body)
            .await
    }

    // -----------------------------------------------------------------------
    // Stablecoins
    // -----------------------------------------------------------------------

    pub async fn stablecoins(&self) -> Result<RawStablecoinsResponse, HeatError> {
        self.get_json(ApiFamily::Stablecoins, "stablecoins", &[])
            .await
    }

    pub async fn stablecoin(&self, id: &str) -> Result<RawStablecoinDetail, HeatError> {
        self.get_json(ApiFamily::Stablecoins, &format!("stablecoin/{id}"), &[])
            .await
    }

    pub async fn stablecoin_chains(&self) -> Result<Vec<RawStablecoinChain>, HeatError> {
        self.get_json(ApiFamily::Stablecoins, "stablecoinchains", &[])
            .await
    }

    pub async fn stablecoin_charts(
        &self,
        chain: Option<&str>,
    ) -> Result<Vec<RawStablecoinChartPoint>, HeatError> {
        let path = match chain {
            Some(c) => format!("stablecoincharts/{c}"),
            None => "stablecoincharts/all".to_owned(),
        };
        self.get_json(ApiFamily::Stablecoins, &path, &[]).await
    }

    pub async fn stablecoin_dominance(
        &self,
        chain: &str,
    ) -> Result<Vec<RawStablecoinDominancePoint>, HeatError> {
        self.get_json(
            ApiFamily::Stablecoins,
            &format!("stablecoindominance/{chain}"),
            &[],
        )
        .await
    }

    pub async fn stablecoin_prices(&self) -> Result<Vec<RawStablecoinPricePoint>, HeatError> {
        self.get_json(ApiFamily::Stablecoins, "stablecoinprices", &[])
            .await
    }

    // -----------------------------------------------------------------------
    // Bridges
    // -----------------------------------------------------------------------

    pub async fn bridges(&self) -> Result<RawBridgesResponse, HeatError> {
        self.get_json(ApiFamily::Bridges, "bridges", &[("includeChains", "true")])
            .await
    }

    pub async fn bridge(&self, id: u64) -> Result<RawBridgeDetail, HeatError> {
        self.get_json(ApiFamily::Bridges, &format!("bridge/{id}"), &[])
            .await
    }

    pub async fn bridge_volume(&self, chain: &str) -> Result<Vec<RawBridgeVolumePoint>, HeatError> {
        self.get_json(ApiFamily::Bridges, &format!("bridgevolume/{chain}"), &[])
            .await
    }

    pub async fn bridge_daystats(
        &self,
        timestamp: i64,
        chain: &str,
    ) -> Result<Vec<RawBridgeDayStats>, HeatError> {
        self.get_json(
            ApiFamily::Bridges,
            &format!("bridgedaystats/{timestamp}/{chain}"),
            &[],
        )
        .await
    }

    pub async fn bridge_transactions(&self, id: u64) -> Result<Vec<RawBridgeTx>, HeatError> {
        self.get_json(ApiFamily::Bridges, &format!("transactions/{id}"), &[])
            .await
    }

    // -----------------------------------------------------------------------
    // Fees
    // -----------------------------------------------------------------------

    pub async fn fees_overview(
        &self,
        chain: Option<&str>,
    ) -> Result<RawOverviewResponse, HeatError> {
        let path = match chain {
            Some(c) => format!("overview/fees/{c}"),
            None => "overview/fees".to_owned(),
        };
        self.get_json(ApiFamily::Main, &path, &[]).await
    }

    // -----------------------------------------------------------------------
    // Volumes
    // -----------------------------------------------------------------------

    pub async fn volumes_dexs(
        &self,
        chain: Option<&str>,
    ) -> Result<RawOverviewResponse, HeatError> {
        let path = match chain {
            Some(c) => format!("overview/dexs/{c}"),
            None => "overview/dexs".to_owned(),
        };
        self.get_json(ApiFamily::Main, &path, &[]).await
    }

    pub async fn fees_protocol(&self, protocol: &str) -> Result<RawSummaryResponse, HeatError> {
        self.get_json(ApiFamily::Main, &format!("summary/fees/{protocol}"), &[])
            .await
    }

    pub async fn volumes_dex_summary(
        &self,
        protocol: &str,
    ) -> Result<RawSummaryResponse, HeatError> {
        self.get_json(ApiFamily::Main, &format!("summary/dexs/{protocol}"), &[])
            .await
    }

    pub async fn volumes_options(
        &self,
        chain: Option<&str>,
    ) -> Result<RawOverviewResponse, HeatError> {
        let path = match chain {
            Some(c) => format!("overview/options/{c}"),
            None => "overview/options".to_owned(),
        };
        self.get_json(ApiFamily::Main, &path, &[]).await
    }

    pub async fn volumes_option_summary(
        &self,
        protocol: &str,
    ) -> Result<RawSummaryResponse, HeatError> {
        self.get_json(ApiFamily::Main, &format!("summary/options/{protocol}"), &[])
            .await
    }

    pub async fn volumes_derivatives(
        &self,
        chain: Option<&str>,
    ) -> Result<RawOverviewResponse, HeatError> {
        let path = match chain {
            Some(c) => format!("overview/derivatives/{c}"),
            None => "overview/derivatives".to_owned(),
        };
        self.get_json(ApiFamily::Main, &path, &[]).await
    }

    pub async fn volumes_derivative_summary(
        &self,
        protocol: &str,
    ) -> Result<RawSummaryResponse, HeatError> {
        self.get_json(
            ApiFamily::Main,
            &format!("summary/derivatives/{protocol}"),
            &[],
        )
        .await
    }

    pub async fn volumes_open_interest(&self) -> Result<RawOverviewResponse, HeatError> {
        self.get_json(ApiFamily::Main, "overview/open-interest", &[])
            .await
    }

    // -----------------------------------------------------------------------
    // Yields
    // -----------------------------------------------------------------------

    pub async fn yields_pools(&self) -> Result<RawYieldsResponse<RawYieldPool>, HeatError> {
        self.get_json(ApiFamily::Yields, "pools", &[]).await
    }

    pub async fn yields_pools_old(&self) -> Result<RawYieldsResponse<RawYieldPool>, HeatError> {
        self.get_json(ApiFamily::Yields, "poolsOld", &[]).await
    }

    pub async fn yields_chart(
        &self,
        pool: &str,
    ) -> Result<RawYieldsResponse<RawYieldChartPoint>, HeatError> {
        self.get_json(ApiFamily::Yields, &format!("chart/{pool}"), &[])
            .await
    }

    pub async fn yields_borrow(&self) -> Result<RawYieldsResponse<RawYieldBorrowPool>, HeatError> {
        self.get_json(ApiFamily::Yields, "poolsBorrow", &[]).await
    }

    pub async fn yields_lend_borrow_chart(
        &self,
        pool: &str,
    ) -> Result<RawYieldsResponse<RawYieldLendBorrowChartPoint>, HeatError> {
        self.get_json(ApiFamily::Yields, &format!("chartLendBorrow/{pool}"), &[])
            .await
    }

    pub async fn yields_perps(&self) -> Result<RawYieldsResponse<RawPerp>, HeatError> {
        self.get_json(ApiFamily::Yields, "perps", &[]).await
    }

    pub async fn yields_lsd(&self) -> Result<RawYieldsResponse<RawLsdRate>, HeatError> {
        self.get_json(ApiFamily::Yields, "lsdRates", &[]).await
    }

    // -----------------------------------------------------------------------
    // Ecosystem
    // -----------------------------------------------------------------------

    pub async fn categories(&self) -> Result<RawCategoriesResponse, HeatError> {
        self.get_json(ApiFamily::Main, "categories", &[]).await
    }

    pub async fn forks(&self) -> Result<RawForksResponse, HeatError> {
        self.get_json(ApiFamily::Main, "forks", &[]).await
    }

    pub async fn oracles(&self) -> Result<RawOraclesResponse, HeatError> {
        self.get_json(ApiFamily::Main, "oracles", &[]).await
    }

    pub async fn entities(&self) -> Result<Vec<RawEntity>, HeatError> {
        self.get_json(ApiFamily::Main, "entities", &[]).await
    }

    pub async fn raises(&self) -> Result<RawRaisesResponse, HeatError> {
        self.get_json(ApiFamily::Main, "raises", &[]).await
    }

    pub async fn treasuries(&self) -> Result<Vec<RawTreasury>, HeatError> {
        self.get_json(ApiFamily::Main, "treasuries", &[]).await
    }

    pub async fn hacks(&self) -> Result<Vec<RawHack>, HeatError> {
        self.get_json(ApiFamily::Main, "hacks", &[]).await
    }

    // -----------------------------------------------------------------------
    // Pro-only endpoints (require API key)
    // -----------------------------------------------------------------------

    pub async fn token_protocols(&self, symbol: &str) -> Result<Vec<RawTokenProtocol>, HeatError> {
        self.get_json(
            ApiFamily::Main,
            &format!("tokenProtocols/{symbol}"),
            &[],
        )
        .await
    }

    pub async fn inflows(
        &self,
        protocol: &str,
        timestamp: i64,
    ) -> Result<RawInflowsResponse, HeatError> {
        self.get_json(
            ApiFamily::Main,
            &format!("inflows/{protocol}/{timestamp}"),
            &[],
        )
        .await
    }

    pub async fn institutions(&self) -> Result<RawInstitutionsResponse, HeatError> {
        self.get_json(ApiFamily::Main, "dat/institutions", &[])
            .await
    }

    pub async fn institution_symbol(&self, symbol: &str) -> Result<RawInstitutionDetail, HeatError> {
        self.get_json(ApiFamily::Main, &format!("dat/institutions/{symbol}"), &[])
            .await
    }

    pub async fn etfs_snapshot(&self) -> Result<Vec<RawEtfSnapshot>, HeatError> {
        self.get_json(ApiFamily::Main, "etfs/snapshot", &[]).await
    }

    pub async fn etfs_flows(&self) -> Result<Vec<RawEtfFlow>, HeatError> {
        self.get_json(ApiFamily::Main, "etfs/flows", &[]).await
    }

    pub async fn fdv_performance(&self, period: &str) -> Result<Vec<serde_json::Value>, HeatError> {
        self.get_json(ApiFamily::Main, &format!("fdv/performance/{period}"), &[])
            .await
    }

    // -----------------------------------------------------------------------
    // Usage (requires API key)
    // -----------------------------------------------------------------------

    pub async fn usage(&self, api_key: &str) -> Result<RawUsage, HeatError> {
        self.get_json(ApiFamily::Main, &format!("usage/{api_key}"), &[])
            .await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_public_main() {
        let c = DefiLlamaClient::new(None).unwrap();
        assert_eq!(
            c.url(ApiFamily::Main, "protocols"),
            "https://api.llama.fi/protocols"
        );
    }

    #[test]
    fn url_public_coins() {
        let c = DefiLlamaClient::new(None).unwrap();
        assert_eq!(
            c.url(ApiFamily::Coins, "prices/current/ethereum:0xabc"),
            "https://coins.llama.fi/prices/current/ethereum:0xabc"
        );
    }

    #[test]
    fn url_public_stablecoins() {
        let c = DefiLlamaClient::new(None).unwrap();
        assert_eq!(
            c.url(ApiFamily::Stablecoins, "stablecoins"),
            "https://stablecoins.llama.fi/stablecoins"
        );
    }

    #[test]
    fn url_public_bridges() {
        let c = DefiLlamaClient::new(None).unwrap();
        assert_eq!(
            c.url(ApiFamily::Bridges, "bridges"),
            "https://bridges.llama.fi/bridges"
        );
    }

    #[test]
    fn url_pro_main() {
        let c = DefiLlamaClient::new(Some("my-key".to_owned())).unwrap();
        assert_eq!(
            c.url(ApiFamily::Main, "protocols"),
            "https://pro-api.llama.fi/my-key/protocols"
        );
    }

    #[test]
    fn url_pro_coins() {
        let c = DefiLlamaClient::new(Some("my-key".to_owned())).unwrap();
        assert_eq!(
            c.url(ApiFamily::Coins, "prices/current/ethereum:0xabc"),
            "https://pro-api.llama.fi/my-key/prices/current/ethereum:0xabc"
        );
    }

    #[test]
    fn url_strips_leading_slash() {
        let c = DefiLlamaClient::new(None).unwrap();
        assert_eq!(
            c.url(ApiFamily::Main, "/protocols"),
            "https://api.llama.fi/protocols"
        );
    }

    #[test]
    fn parse_iso_timestamp() {
        assert_eq!(super::parse_iso_to_unix("2026-03-10T23:16:23.000Z").unwrap(), 1773184583);
    }

    #[test]
    fn bridge_tx_deserialize_string_ts() {
        let json = r#"{"tx_hash":"0xabc","ts":"2026-03-10T23:16:23.000Z"}"#;
        let tx: super::RawBridgeTx = serde_json::from_str(json).unwrap();
        assert_eq!(tx.ts, Some(1773184583));
    }

    #[test]
    fn bridge_tx_deserialize_int_ts() {
        let json = r#"{"tx_hash":"0xabc","ts":1704067200}"#;
        let tx: super::RawBridgeTx = serde_json::from_str(json).unwrap();
        assert_eq!(tx.ts, Some(1704067200));
    }

    #[test]
    fn url_test_override() {
        let c = DefiLlamaClient::with_base_url("http://localhost:8080").unwrap();
        assert_eq!(
            c.url(ApiFamily::Main, "protocols"),
            "http://localhost:8080/protocols"
        );
        // Override ignores family
        assert_eq!(
            c.url(ApiFamily::Coins, "prices/current/ethereum:0xabc"),
            "http://localhost:8080/prices/current/ethereum:0xabc"
        );
    }
}
