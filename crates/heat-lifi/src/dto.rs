/// Heat-owned DTOs for LI.FI protocol output.
///
/// These types are the stable public contract for `heat lifi` commands.
/// Raw LI.FI API responses are mapped into these types in `map.rs` and
/// never surfaced directly.
use serde::Serialize;

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct TokenDto {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
    pub name: String,
    pub chain_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_uri: Option<String>,
}

// ---------------------------------------------------------------------------
// Chain
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ChainDto {
    pub id: u64,
    pub name: String,
    pub chain_type: String, // "EVM", "SVM", etc.
    pub native_token: TokenDto,
}

// ---------------------------------------------------------------------------
// Tool (bridge or DEX)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct BridgeToolDto {
    pub key: String,
    pub name: String,
    pub logo_uri: Option<String>,
    pub supported_chains: Vec<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExchangeToolDto {
    pub key: String,
    pub name: String,
    pub logo_uri: Option<String>,
    pub supported_chains: Vec<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolsDto {
    pub bridges: Vec<BridgeToolDto>,
    pub exchanges: Vec<ExchangeToolDto>,
}

// ---------------------------------------------------------------------------
// Fee / gas estimate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct FeeDto {
    pub amount: String,
    pub token: TokenDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct EstimateDto {
    pub from_amount: String,
    pub to_amount: String,
    pub to_amount_min: String,
    /// Approximate execution time in seconds.
    pub execution_duration: u64,
    pub fees: Vec<FeeDto>,
}

// ---------------------------------------------------------------------------
// Quote
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct QuoteDto {
    pub from_chain_id: u64,
    pub to_chain_id: u64,
    pub from_token: TokenDto,
    pub to_token: TokenDto,
    pub from_amount: String,
    pub to_amount: String,
    pub to_amount_min: String,
    /// Tool (bridge or DEX) used for this quote.
    pub tool: String,
    pub tool_details: ToolDetailsDto,
    pub estimate: EstimateDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDetailsDto {
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_uri: Option<String>,
}

// ---------------------------------------------------------------------------
// Route / step
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct StepDto {
    pub step_type: String, // "swap", "cross", etc.
    pub tool: String,
    pub from_token: TokenDto,
    pub to_token: TokenDto,
    pub from_amount: String,
    pub to_amount: String,
    pub estimate: EstimateDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteDto {
    pub id: String,
    pub from_chain_id: u64,
    pub to_chain_id: u64,
    pub from_token: TokenDto,
    pub to_token: TokenDto,
    pub from_amount: String,
    pub to_amount: String,
    pub to_amount_min: String,
    pub steps: Vec<StepDto>,
    /// Tags such as "CHEAPEST", "FASTEST", "RECOMMENDED".
    pub tags: Vec<String>,
    /// Whether Heat can execute this route today.
    pub execution_supported: bool,
    /// Execution chain family (e.g. "EVM", "Solana").
    pub execution_family: String,
    /// Reason when execution is not supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct StatusDto {
    pub status: String,    // "DONE", "PENDING", "FAILED", etc.
    pub substatus: String, // finer-grained status
    pub tx_hash: Option<String>,
    pub sending_tx_hash: Option<String>,
    pub receiving_tx_hash: Option<String>,
    pub from_chain_id: Option<u64>,
    pub to_chain_id: Option<u64>,
    pub from_token: Option<TokenDto>,
    pub to_token: Option<TokenDto>,
    pub from_amount: Option<String>,
    pub to_amount: Option<String>,
}

// ---------------------------------------------------------------------------
// Collections (used as top-level write_data payloads)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ChainsListDto {
    pub chains: Vec<ChainDto>,
}

#[derive(Debug, Serialize)]
pub struct TokensListDto {
    pub tokens: Vec<TokenDto>,
    pub chain_id: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct RoutesListDto {
    pub routes: Vec<RouteDto>,
    pub from_chain_id: u64,
    pub to_chain_id: u64,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
}

// ---------------------------------------------------------------------------
// Bridge result
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct BridgeResultDto {
    pub from_chain: String,
    pub to_chain: String,
    pub from_token: TokenDto,
    pub to_token: TokenDto,
    pub from_amount: String,
    pub from_amount_display: String,
    pub to_amount_estimate: String,
    pub to_amount_min: String,
    pub route_id: String,
    pub route_tags: Vec<String>,
    pub tools_used: Vec<String>,
    pub account: String,
    pub step_results: Vec<StepResultDto>,
}

#[derive(Debug, Serialize)]
pub struct StepResultDto {
    pub step_type: String,
    pub tool: String,
    pub tx_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_tx_hash: Option<String>,
}
