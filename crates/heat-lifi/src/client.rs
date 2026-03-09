/// LI.FI REST API client.
///
/// Thin, typed wrapper over the public LI.FI API at `https://li.quest/v1/`.
/// All HTTP and JSON errors are mapped to `HeatError` at this boundary.
/// Callers receive typed response structs that `map.rs` then converts to
/// Heat-owned DTOs.
use heat_core::error::HeatError;
use serde::{Deserialize, Serialize};

pub const LIFI_BASE_URL: &str = "https://li.quest/v1";

// ---------------------------------------------------------------------------
// Raw API response shapes (internal — not part of Heat's output contract)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct RawToken {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
    pub name: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    #[serde(rename = "logoURI")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawChain {
    pub id: u64,
    pub name: String,
    #[serde(rename = "chainType")]
    pub chain_type: String,
    #[serde(rename = "nativeToken")]
    pub native_token: RawToken,
}

#[derive(Debug, Deserialize)]
pub struct ChainsResponse {
    pub chains: Vec<RawChain>,
}

#[derive(Debug, Deserialize)]
pub struct TokensResponse {
    /// Map of chainId → list of tokens.
    pub tokens: std::collections::HashMap<String, Vec<RawToken>>,
}

#[derive(Debug, Deserialize)]
pub struct RawBridgeTool {
    pub key: String,
    pub name: String,
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
    #[serde(rename = "supportedChains", default)]
    pub supported_chains: Vec<RawSupportedChain>,
}

#[derive(Debug, Deserialize)]
pub struct RawExchangeTool {
    pub key: String,
    pub name: String,
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
    #[serde(rename = "supportedChains", default)]
    pub supported_chains: Vec<RawSupportedChain>,
}

#[derive(Debug, Deserialize)]
pub struct RawSupportedChain {
    #[serde(rename = "chainId")]
    pub chain_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct ToolsResponse {
    pub bridges: Vec<RawBridgeTool>,
    pub exchanges: Vec<RawExchangeTool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawToolDetails {
    pub key: String,
    pub name: String,
    #[serde(rename = "logoURI")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_uri: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawFee {
    pub amount: String,
    pub token: RawToken,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawEstimate {
    #[serde(rename = "fromAmount")]
    pub from_amount: String,
    #[serde(rename = "toAmount")]
    pub to_amount: String,
    #[serde(rename = "toAmountMin")]
    pub to_amount_min: String,
    #[serde(rename = "executionDuration")]
    pub execution_duration: f64,
    #[serde(rename = "feeCosts", default)]
    pub fee_costs: Vec<RawFee>,
    /// Spender address that needs ERC-20 approval (absent for native tokens).
    #[serde(rename = "approvalAddress")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_address: Option<String>,
}

/// A LI.FI step (used both in quotes and route steps).
#[derive(Debug, Deserialize, Serialize)]
pub struct RawStep {
    #[serde(rename = "type")]
    pub step_type: String,
    pub tool: String,
    #[serde(rename = "toolDetails")]
    pub tool_details: RawToolDetails,
    pub action: RawStepAction,
    pub estimate: RawEstimate,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawStepAction {
    #[serde(rename = "fromToken")]
    pub from_token: RawToken,
    #[serde(rename = "toToken")]
    pub to_token: RawToken,
    #[serde(rename = "fromAmount")]
    pub from_amount: String,
    #[serde(rename = "fromChainId")]
    pub from_chain_id: u64,
    #[serde(rename = "toChainId")]
    pub to_chain_id: u64,
    /// Sender address — required by `/advanced/stepTransaction`.
    /// Absent in route responses; must be injected before calling stepTransaction.
    #[serde(rename = "fromAddress")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_address: Option<String>,
}

/// LI.FI returns the quote as a single step object.
pub type QuoteResponse = RawStep;

#[derive(Debug, Deserialize)]
pub struct RawRoute {
    pub id: String,
    #[serde(rename = "fromChainId")]
    pub from_chain_id: u64,
    #[serde(rename = "toChainId")]
    pub to_chain_id: u64,
    #[serde(rename = "fromToken")]
    pub from_token: RawToken,
    #[serde(rename = "toToken")]
    pub to_token: RawToken,
    #[serde(rename = "fromAmount")]
    pub from_amount: String,
    #[serde(rename = "toAmount")]
    pub to_amount: String,
    #[serde(rename = "toAmountMin")]
    pub to_amount_min: String,
    pub steps: Vec<RawStep>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RoutesResponse {
    pub routes: Vec<RawRoute>,
}

#[derive(Debug, Deserialize)]
pub struct RawStatus {
    pub status: String,
    #[serde(default)]
    pub substatus: String,
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
    #[serde(rename = "sending")]
    pub sending: Option<RawStatusTxInfo>,
    #[serde(rename = "receiving")]
    pub receiving: Option<RawStatusTxInfo>,
    #[serde(rename = "fromChainId")]
    pub from_chain_id: Option<u64>,
    #[serde(rename = "toChainId")]
    pub to_chain_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct RawStatusTxInfo {
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
    pub token: Option<RawToken>,
    pub amount: Option<String>,
}

pub type StatusResponse = RawStatus;

// ---------------------------------------------------------------------------
// Step transaction (execution)
// ---------------------------------------------------------------------------

/// Raw transaction request returned by LI.FI for a step.
#[derive(Debug, Deserialize)]
pub struct RawTransactionRequest {
    pub to: String,
    pub data: String,
    pub value: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    #[serde(rename = "gasLimit")]
    pub gas_limit: Option<String>,
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<String>,
}

/// Response from POST /advanced/stepTransaction.
///
/// The response is the step enriched with `transactionRequest`.
#[derive(Debug, Deserialize)]
pub struct StepTransactionResponse {
    #[serde(rename = "transactionRequest")]
    pub transaction_request: RawTransactionRequest,
    pub estimate: RawEstimate,
    pub action: RawStepAction,
}

// ---------------------------------------------------------------------------
// Request parameter structs
// ---------------------------------------------------------------------------

pub struct QuoteParams {
    pub from_chain: String,
    pub to_chain: String,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub from_address: Option<String>,
}

pub struct RoutesParams {
    pub from_chain_id: u64,
    pub to_chain_id: u64,
    pub from_token_address: String,
    pub to_token_address: String,
    pub from_amount: String,
    pub from_address: Option<String>,
}

pub struct StatusParams {
    pub tx_hash: String,
    pub bridge: Option<String>,
    pub from_chain: String,
    pub to_chain: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct LifiClient {
    base_url: String,
    client: reqwest::Client,
    api_key: Option<String>,
}

impl LifiClient {
    pub fn new() -> Result<Self, HeatError> {
        let api_key = std::env::var("HEAT_LIFI_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let client = reqwest::Client::builder()
            .user_agent("heat-cli/0.1")
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                HeatError::internal("http_client", format!("Failed to build HTTP client: {e}"))
            })?;
        Ok(Self {
            base_url: LIFI_BASE_URL.to_owned(),
            client,
            api_key,
        })
    }

    /// Construct with a custom base URL (useful in tests / staging environments).
    #[allow(dead_code)]
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self, HeatError> {
        let client = reqwest::Client::builder()
            .user_agent("heat-cli/0.1")
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| {
                HeatError::internal("http_client", format!("Failed to build HTTP client: {e}"))
            })?;
        Ok(Self {
            base_url: base_url.into(),
            client,
            api_key: None,
        })
    }

    /// Override the API key after construction. Intended for tests.
    #[allow(dead_code)]
    pub fn set_api_key(&mut self, key: impl Into<String>) {
        self.api_key = Some(key.into());
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    pub(crate) fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, HeatError> {
        let url = self.url(path);
        let mut req = self.client.get(&url).query(query);
        if let Some(key) = &self.api_key {
            req = req.header("x-lifi-api-key", key);
        }
        let resp = req.send().await.map_err(|e| {
            HeatError::network("request_failed", format!("LI.FI request failed: {e}"))
        })?;

        let status = resp.status();
        if !status.is_success() {
            // Try to extract an error message from the body.
            let body = resp.text().await.unwrap_or_default();
            let msg = format!("LI.FI API returned {status}: {body}");
            return Err(if status.as_u16() == 429 || status.is_server_error() {
                HeatError::network("api_error", msg)
            } else {
                HeatError::protocol("api_error", msg)
            });
        }

        resp.json::<T>().await.map_err(|e| {
            HeatError::protocol(
                "parse_error",
                format!("Failed to parse LI.FI response: {e}"),
            )
        })
    }

    async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, HeatError> {
        let url = self.url(path);
        let mut req = self.client.post(&url).json(body);
        if let Some(key) = &self.api_key {
            req = req.header("x-lifi-api-key", key);
        }
        let resp = req.send().await.map_err(|e| {
            HeatError::network("request_failed", format!("LI.FI request failed: {e}"))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            let msg = format!("LI.FI API returned {status}: {body_text}");
            return Err(if status.as_u16() == 429 || status.is_server_error() {
                HeatError::network("api_error", msg)
            } else {
                HeatError::protocol("api_error", msg)
            });
        }

        resp.json::<T>().await.map_err(|e| {
            HeatError::protocol(
                "parse_error",
                format!("Failed to parse LI.FI response: {e}"),
            )
        })
    }

    // -----------------------------------------------------------------------
    // Public API methods
    // -----------------------------------------------------------------------

    pub async fn chains(&self) -> Result<ChainsResponse, HeatError> {
        self.get_json("chains", &[]).await
    }

    pub async fn tokens(&self, chain_id: Option<u64>) -> Result<TokensResponse, HeatError> {
        let chain_id_str = chain_id.map(|id| id.to_string());
        let query: Vec<(&str, &str)> = chain_id_str
            .as_deref()
            .map(|id| vec![("chains", id)])
            .unwrap_or_default();
        self.get_json("tokens", &query).await
    }

    pub async fn tools(&self) -> Result<ToolsResponse, HeatError> {
        self.get_json("tools", &[]).await
    }

    pub async fn quote(&self, params: &QuoteParams) -> Result<QuoteResponse, HeatError> {
        let mut query: Vec<(&str, &str)> = vec![
            ("fromChain", params.from_chain.as_str()),
            ("toChain", params.to_chain.as_str()),
            ("fromToken", params.from_token.as_str()),
            ("toToken", params.to_token.as_str()),
            ("fromAmount", params.from_amount.as_str()),
        ];
        if let Some(addr) = &params.from_address {
            query.push(("fromAddress", addr.as_str()));
        }
        self.get_json("quote", &query).await
    }

    pub async fn routes(&self, params: &RoutesParams) -> Result<RoutesResponse, HeatError> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct RoutesBody {
            from_chain_id: u64,
            to_chain_id: u64,
            from_token_address: String,
            to_token_address: String,
            from_amount: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            from_address: Option<String>,
        }

        let body = RoutesBody {
            from_chain_id: params.from_chain_id,
            to_chain_id: params.to_chain_id,
            from_token_address: params.from_token_address.clone(),
            to_token_address: params.to_token_address.clone(),
            from_amount: params.from_amount.clone(),
            from_address: params.from_address.clone(),
        };

        self.post_json("advanced/routes", &body).await
    }

    pub async fn status(&self, params: &StatusParams) -> Result<StatusResponse, HeatError> {
        let mut query: Vec<(&str, &str)> = vec![
            ("txHash", params.tx_hash.as_str()),
            ("fromChain", params.from_chain.as_str()),
            ("toChain", params.to_chain.as_str()),
        ];
        if let Some(bridge) = &params.bridge {
            query.push(("bridge", bridge.as_str()));
        }
        self.get_json("status", &query).await
    }

    /// Request transaction data for a single route step.
    ///
    /// The step should be a raw JSON value from a routes response,
    /// enriched with the sender's address in `action.fromAddress`.
    pub async fn step_transaction(
        &self,
        step: &serde_json::Value,
    ) -> Result<StepTransactionResponse, HeatError> {
        self.post_json("advanced/stepTransaction", step).await
    }
}
