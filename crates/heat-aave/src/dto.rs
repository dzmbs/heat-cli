/// Heat-owned output types for Aave V3.
///
/// These DTOs are the public output contract. They never leak raw
/// contract return types. Money-sensitive values use strings.
use serde::Serialize;

// ---------------------------------------------------------------------------
// Markets
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct MarketsListDto {
    pub chain: String,
    pub markets: Vec<MarketDto>,
}

#[derive(Debug, Serialize)]
pub struct MarketDto {
    pub symbol: String,
    pub underlying_address: String,
    pub decimals: u8,
    pub a_token_address: String,
    pub variable_debt_token_address: String,
    pub collateral_enabled: bool,
    pub borrowing_enabled: bool,
    pub is_active: bool,
    pub is_frozen: bool,
    pub is_paused: bool,
    /// Supply cap in token units (0 = uncapped).
    pub supply_cap: String,
    /// Borrow cap in token units (0 = uncapped).
    pub borrow_cap: String,
    /// Total supplied (aToken total supply) in base units.
    pub total_supplied: String,
    /// Total stable debt in base units.
    pub total_stable_debt: String,
    /// Total variable debt in base units.
    pub total_variable_debt: String,
    /// Supply APY as a percentage string (e.g. "3.45").
    pub supply_apy: String,
    /// Variable borrow APY as a percentage string.
    pub variable_borrow_apy: String,
    /// Loan-to-value ratio in basis points.
    pub ltv_bps: u64,
    /// Liquidation threshold in basis points.
    pub liquidation_threshold_bps: u64,
}

// ---------------------------------------------------------------------------
// Positions
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PositionsListDto {
    pub chain: String,
    pub account: String,
    pub positions: Vec<PositionDto>,
}

#[derive(Debug, Serialize)]
pub struct PositionDto {
    pub symbol: String,
    pub underlying_address: String,
    pub decimals: u8,
    /// Supplied balance (aToken balance) in base units.
    pub supplied: String,
    /// Stable debt in base units.
    pub stable_debt: String,
    /// Variable debt in base units.
    pub variable_debt: String,
    pub collateral_enabled: bool,
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct HealthDto {
    pub chain: String,
    pub account: String,
    /// Total collateral in the market's base currency (USD, 8 decimals on Aave V3).
    pub total_collateral_base: String,
    /// Total debt in base currency.
    pub total_debt_base: String,
    /// Available borrows in base currency.
    pub available_borrows_base: String,
    /// Current liquidation threshold in basis points.
    pub liquidation_threshold_bps: u64,
    /// Loan-to-value in basis points.
    pub ltv_bps: u64,
    /// Health factor (WAD-scaled, 1e18 = 1.0). String for exactness.
    pub health_factor: String,
    /// Human-readable health factor (e.g. "1.85").
    pub health_factor_display: String,
}

// ---------------------------------------------------------------------------
// Supply result
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SupplyResultDto {
    pub chain: String,
    pub account: String,
    pub asset_symbol: String,
    pub asset_address: String,
    pub amount: String,
    pub amount_display: String,
    pub tx_hash: String,
    pub approval_tx_hash: Option<String>,
    /// "erc20" for standard token supply, "native_gateway" for ETH via WETH Gateway.
    pub execution_path: String,
    /// WETH Gateway address when execution_path is "native_gateway", null otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_address: Option<String>,
}

// ---------------------------------------------------------------------------
// Withdraw result
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct WithdrawResultDto {
    pub chain: String,
    pub account: String,
    pub asset_symbol: String,
    pub asset_address: String,
    pub amount_requested: String,
    pub amount_requested_display: String,
    pub tx_hash: String,
    /// "erc20" for standard token withdraw, "native_gateway" for ETH via WETH Gateway.
    pub execution_path: String,
    /// WETH Gateway address when execution_path is "native_gateway", null otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_address: Option<String>,
    /// aWETH approval tx hash (needed for native withdraw via gateway).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_tx_hash: Option<String>,
}
