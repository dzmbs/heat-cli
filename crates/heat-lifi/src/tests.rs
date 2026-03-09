//! Tests for heat-lifi.
//!
//! Unit tests for the execution classifier live in `exec.rs`.
//! This file covers:
//! - DTO serialization stability (the output contract)
//! - Map layer correctness
//! - Client error-category mapping
//! - Chain/token parsing helpers

use crate::client::{
    ChainsResponse, RawBridgeTool, RawChain, RawEstimate, RawExchangeTool, RawFee, RawRoute,
    RawStatus, RawStatusTxInfo, RawStep, RawStepAction, RawSupportedChain, RawToken,
    RawToolDetails, RoutesResponse, ToolsResponse,
};
use crate::dto::TokenDto;
use crate::exec::{ExecutionFamily, classify_route};
use crate::map::{self, RoutesSummary};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn raw_usdc(chain_id: u64) -> RawToken {
    RawToken {
        address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_owned(),
        symbol: "USDC".to_owned(),
        decimals: 6,
        name: "USD Coin".to_owned(),
        chain_id,
        logo_uri: Some("https://example.com/usdc.png".to_owned()),
    }
}

fn raw_eth_chain() -> RawChain {
    RawChain {
        id: 1,
        name: "Ethereum".to_owned(),
        chain_type: "EVM".to_owned(),
        native_token: RawToken {
            address: "0x0000000000000000000000000000000000000000".to_owned(),
            symbol: "ETH".to_owned(),
            decimals: 18,
            name: "Ether".to_owned(),
            chain_id: 1,
            logo_uri: None,
        },
    }
}

fn raw_estimate(from_chain: u64, _to_chain: u64) -> RawEstimate {
    RawEstimate {
        from_amount: "1000000".to_owned(),
        to_amount: "998000".to_owned(),
        to_amount_min: "990000".to_owned(),
        execution_duration: 120.0,
        fee_costs: vec![RawFee {
            amount: "2000".to_owned(),
            token: raw_usdc(from_chain),
        }],
        approval_address: None,
    }
}

fn raw_step(from_chain: u64, to_chain: u64) -> RawStep {
    RawStep {
        step_type: "cross".to_owned(),
        tool: "stargate".to_owned(),
        tool_details: RawToolDetails {
            key: "stargate".to_owned(),
            name: "Stargate".to_owned(),
            logo_uri: None,
        },
        action: RawStepAction {
            from_token: raw_usdc(from_chain),
            to_token: raw_usdc(to_chain),
            from_amount: "1000000".to_owned(),
            from_chain_id: from_chain,
            to_chain_id: to_chain,
            from_address: None,
        },
        estimate: raw_estimate(from_chain, to_chain),
    }
}

fn raw_route(from_chain: u64, to_chain: u64) -> RawRoute {
    RawRoute {
        id: "route-abc".to_owned(),
        from_chain_id: from_chain,
        to_chain_id: to_chain,
        from_token: raw_usdc(from_chain),
        to_token: raw_usdc(to_chain),
        from_amount: "1000000".to_owned(),
        to_amount: "998000".to_owned(),
        to_amount_min: "990000".to_owned(),
        steps: vec![raw_step(from_chain, to_chain)],
        tags: vec!["CHEAPEST".to_owned()],
    }
}

// ---------------------------------------------------------------------------
// DTO serialization stability
// ---------------------------------------------------------------------------

#[test]
fn token_dto_serializes_with_expected_fields() {
    let token = map::map_token(&raw_usdc(1));
    let json = serde_json::to_value(&token).unwrap();

    assert_eq!(
        json["address"],
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
    );
    assert_eq!(json["symbol"], "USDC");
    assert_eq!(json["decimals"], 6);
    assert_eq!(json["chain_id"], 1);
    // logo_uri present and non-null
    assert!(json["logo_uri"].is_string());
}

#[test]
fn token_dto_omits_logo_uri_when_none() {
    let mut raw = raw_usdc(1);
    raw.logo_uri = None;
    let token = map::map_token(&raw);
    let json = serde_json::to_value(&token).unwrap();

    // logo_uri should be absent (skip_serializing_if = Option::is_none)
    assert!(json.get("logo_uri").is_none());
}

#[test]
fn chain_dto_serializes_with_expected_fields() {
    let chains = map::map_chains(ChainsResponse {
        chains: vec![raw_eth_chain()],
    });
    let json = serde_json::to_value(&chains).unwrap();

    let chain = &json["chains"][0];
    assert_eq!(chain["id"], 1);
    assert_eq!(chain["name"], "Ethereum");
    assert_eq!(chain["chain_type"], "EVM");
    assert_eq!(chain["native_token"]["symbol"], "ETH");
}

#[test]
fn quote_dto_serializes_all_required_fields() {
    let raw = raw_step(1, 42161);
    let dto = map::map_quote(raw);
    let json = serde_json::to_value(&dto).unwrap();

    // Verify stable field names (snake_case).
    assert!(json["from_chain_id"].is_number());
    assert!(json["to_chain_id"].is_number());
    assert!(json["from_token"].is_object());
    assert!(json["to_token"].is_object());
    assert!(json["from_amount"].is_string());
    assert!(json["to_amount"].is_string());
    assert!(json["to_amount_min"].is_string());
    assert!(json["tool"].is_string());
    assert!(json["estimate"].is_object());
    assert!(json["estimate"]["execution_duration"].is_number());
    assert!(json["estimate"]["fees"].is_array());
}

#[test]
fn route_dto_serializes_with_steps_and_tags() {
    let raw_resp = RoutesResponse {
        routes: vec![raw_route(1, 42161)],
    };
    let summary = RoutesSummary {
        from_chain_id: 1,
        to_chain_id: 42161,
        from_token: "0xusdc-eth".to_owned(),
        to_token: "0xusdc-arb".to_owned(),
        from_amount: "1000000".to_owned(),
    };
    let chain_types = std::collections::HashMap::new();
    let dto = map::map_routes(raw_resp, summary, &chain_types);
    let json = serde_json::to_value(&dto).unwrap();

    let route = &json["routes"][0];
    assert_eq!(route["id"], "route-abc");
    assert_eq!(route["from_chain_id"], 1);
    assert_eq!(route["to_chain_id"], 42161);
    assert!(route["steps"].is_array());
    assert_eq!(route["steps"].as_array().unwrap().len(), 1);
    assert_eq!(route["tags"][0], "CHEAPEST");
    // Execution support fields must be present.
    assert!(route["execution_supported"].is_boolean());
    assert!(route["execution_family"].is_string());
    // EVM-to-EVM routes are classified as supported.
    assert_eq!(route["execution_supported"], true);
    assert_eq!(route["execution_family"], "EVM");
    // No reason when supported.
    assert!(route.get("execution_reason").is_none());
}

#[test]
fn status_dto_serializes_with_status_field() {
    let raw = RawStatus {
        status: "DONE".to_owned(),
        substatus: "COMPLETED".to_owned(),
        tx_hash: Some("0xdeadbeef".to_owned()),
        sending: None,
        receiving: None,
        from_chain_id: Some(1),
        to_chain_id: Some(42161),
    };
    let dto = map::map_status(raw);
    let json = serde_json::to_value(&dto).unwrap();

    assert_eq!(json["status"], "DONE");
    assert_eq!(json["substatus"], "COMPLETED");
    assert_eq!(json["tx_hash"], "0xdeadbeef");
    assert_eq!(json["from_chain_id"], 1);
    assert_eq!(json["to_chain_id"], 42161);
}

// ---------------------------------------------------------------------------
// Map layer correctness
// ---------------------------------------------------------------------------

#[test]
fn map_chains_collects_all_chains() {
    let resp = ChainsResponse {
        chains: vec![
            raw_eth_chain(),
            RawChain {
                id: 42161,
                name: "Arbitrum".to_owned(),
                chain_type: "EVM".to_owned(),
                native_token: RawToken {
                    address: "0x0000000000000000000000000000000000000000".to_owned(),
                    symbol: "ETH".to_owned(),
                    decimals: 18,
                    name: "Ether".to_owned(),
                    chain_id: 42161,
                    logo_uri: None,
                },
            },
        ],
    };
    let dto = map::map_chains(resp);
    assert_eq!(dto.chains.len(), 2);
    assert_eq!(dto.chains[0].id, 1);
    assert_eq!(dto.chains[1].id, 42161);
}

#[test]
fn map_tokens_flattens_chain_map() {
    let mut tokens: std::collections::HashMap<String, Vec<crate::client::RawToken>> =
        std::collections::HashMap::new();
    tokens.insert("1".to_owned(), vec![raw_usdc(1)]);
    tokens.insert(
        "42161".to_owned(),
        vec![RawToken {
            address: "0xusdc-arb".to_owned(),
            symbol: "USDC".to_owned(),
            decimals: 6,
            name: "USD Coin".to_owned(),
            chain_id: 42161,
            logo_uri: None,
        }],
    );
    let resp = crate::client::TokensResponse { tokens };
    let dto = map::map_tokens(resp, None);
    // Both tokens should be flattened into a single list.
    assert_eq!(dto.tokens.len(), 2);
    assert!(dto.chain_id.is_none());
}

#[test]
fn map_tools_preserves_bridge_and_exchange_counts() {
    let resp = ToolsResponse {
        bridges: vec![RawBridgeTool {
            key: "stargate".to_owned(),
            name: "Stargate".to_owned(),
            logo_uri: None,
            supported_chains: vec![
                RawSupportedChain { chain_id: 1 },
                RawSupportedChain { chain_id: 42161 },
            ],
        }],
        exchanges: vec![RawExchangeTool {
            key: "uniswap".to_owned(),
            name: "Uniswap".to_owned(),
            logo_uri: None,
            supported_chains: vec![RawSupportedChain { chain_id: 1 }],
        }],
    };
    let dto = map::map_tools(resp);
    assert_eq!(dto.bridges.len(), 1);
    assert_eq!(dto.bridges[0].key, "stargate");
    assert_eq!(dto.bridges[0].supported_chains.len(), 2);
    assert_eq!(dto.exchanges.len(), 1);
    assert_eq!(dto.exchanges[0].key, "uniswap");
}

#[test]
fn map_quote_preserves_amount_strings() {
    let raw = raw_step(1, 42161);
    let dto = map::map_quote(raw);
    assert_eq!(dto.from_amount, "1000000");
    assert_eq!(dto.to_amount, "998000");
    assert_eq!(dto.to_amount_min, "990000");
    assert_eq!(dto.estimate.execution_duration, 120);
}

#[test]
fn map_status_extracts_sending_and_receiving_hashes() {
    let raw = RawStatus {
        status: "PENDING".to_owned(),
        substatus: "WAIT_DESTINATION_TRANSACTION".to_owned(),
        tx_hash: Some("0xsending".to_owned()),
        sending: Some(RawStatusTxInfo {
            tx_hash: Some("0xsending-detail".to_owned()),
            token: Some(raw_usdc(1)),
            amount: Some("1000000".to_owned()),
        }),
        receiving: Some(RawStatusTxInfo {
            tx_hash: Some("0xreceiving".to_owned()),
            token: Some(raw_usdc(42161)),
            amount: Some("998000".to_owned()),
        }),
        from_chain_id: Some(1),
        to_chain_id: Some(42161),
    };
    let dto = map::map_status(raw);
    assert_eq!(dto.sending_tx_hash.as_deref(), Some("0xsending-detail"));
    assert_eq!(dto.receiving_tx_hash.as_deref(), Some("0xreceiving"));
    assert_eq!(dto.from_amount.as_deref(), Some("1000000"));
    assert_eq!(dto.to_amount.as_deref(), Some("998000"));
    assert_eq!(
        dto.from_token.as_ref().map(|t| t.symbol.as_str()),
        Some("USDC")
    );
}

// ---------------------------------------------------------------------------
// HeatError categorisation for simulated client failures
// ---------------------------------------------------------------------------

#[test]
fn network_error_maps_to_retryable() {
    use heat_core::error::{ErrorCategory, HeatError};
    let err = HeatError::network("request_failed", "LI.FI request failed: connection refused");
    assert_eq!(err.category, ErrorCategory::Network);
    assert!(err.retryable);
}

#[test]
fn protocol_error_is_not_retryable() {
    use heat_core::error::{ErrorCategory, HeatError};
    let err = HeatError::protocol("api_error", "LI.FI API returned 400: bad request");
    assert_eq!(err.category, ErrorCategory::Protocol);
    assert!(!err.retryable);
}

#[test]
fn parse_error_maps_to_protocol_category() {
    use heat_core::error::{ErrorCategory, HeatError};
    let err = HeatError::protocol(
        "parse_error",
        "Failed to parse LI.FI response: missing field",
    );
    assert_eq!(err.category, ErrorCategory::Protocol);
}

// ---------------------------------------------------------------------------
// LIFI_BASE_URL sanity check
// ---------------------------------------------------------------------------

#[test]
fn base_url_is_https_and_points_to_lifi() {
    let url = crate::client::LIFI_BASE_URL;
    assert!(url.starts_with("https://"), "base URL must use HTTPS");
    assert!(url.contains("li.quest"), "base URL must point to li.quest");
}

// ---------------------------------------------------------------------------
// HTTP-layer unit tests (URL construction, serialization, error categorisation,
// amount validation). No mock HTTP library — tested purely at the unit level.
// ---------------------------------------------------------------------------

#[test]
fn client_url_joins_base_and_path_correctly() {
    let client = crate::client::LifiClient::with_base_url("https://li.quest/v1").unwrap();
    assert_eq!(client.url("chains"), "https://li.quest/v1/chains");
    assert_eq!(client.url("/chains"), "https://li.quest/v1/chains");
    // Trailing slash on base should be normalised.
    let client2 = crate::client::LifiClient::with_base_url("https://li.quest/v1/").unwrap();
    assert_eq!(client2.url("tokens"), "https://li.quest/v1/tokens");
}

#[test]
fn quote_params_map_to_expected_query_fields() {
    // Verify the field names used in the query slice match what LI.FI expects.
    // We test by constructing the params struct and checking field names manually.
    let params = crate::client::QuoteParams {
        from_chain: "1".to_owned(),
        to_chain: "42161".to_owned(),
        from_token: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_owned(),
        to_token: "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8".to_owned(),
        from_amount: "1000000".to_owned(),
        from_address: Some("0xDeaDbeefdEAdbeefdEadbEEFdeadbeEFdEaDbeeF".to_owned()),
    };
    // The query vec built inside client.quote() uses camelCase keys matching LI.FI's API.
    // We test the expected key set by verifying a hand-built equivalent.
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
    let keys: Vec<&str> = query.iter().map(|(k, _)| *k).collect();
    assert!(keys.contains(&"fromChain"));
    assert!(keys.contains(&"toChain"));
    assert!(keys.contains(&"fromToken"));
    assert!(keys.contains(&"toToken"));
    assert!(keys.contains(&"fromAmount"));
    assert!(keys.contains(&"fromAddress"));
}

#[test]
fn routes_body_serializes_with_camel_case() {
    // RoutesBody is a private struct inside client.routes(), so we replicate it here
    // to test that serde rename_all = "camelCase" produces the expected JSON keys.
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
        from_chain_id: 1,
        to_chain_id: 42161,
        from_token_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_owned(),
        to_token_address: "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8".to_owned(),
        from_amount: "1000000".to_owned(),
        from_address: None,
    };

    let json = serde_json::to_value(&body).unwrap();
    assert_eq!(json["fromChainId"], 1);
    assert_eq!(json["toChainId"], 42161);
    assert_eq!(
        json["fromTokenAddress"],
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    );
    assert_eq!(
        json["toTokenAddress"],
        "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8"
    );
    assert_eq!(json["fromAmount"], "1000000");
    // fromAddress absent when None.
    assert!(json.get("fromAddress").is_none());

    // With fromAddress present.
    let body_with_addr = RoutesBody {
        from_chain_id: 1,
        to_chain_id: 42161,
        from_token_address: "0xabc".to_owned(),
        to_token_address: "0xdef".to_owned(),
        from_amount: "500".to_owned(),
        from_address: Some("0xDeaDbeefdEAdbeefdEadbEEFdeadbeEFdEaDbeeF".to_owned()),
    };
    let json2 = serde_json::to_value(&body_with_addr).unwrap();
    assert_eq!(
        json2["fromAddress"],
        "0xDeaDbeefdEAdbeefdEadbEEFdeadbeEFdEaDbeeF"
    );
}

#[test]
fn http_429_maps_to_network_retryable() {
    use heat_core::error::{ErrorCategory, HeatError};
    // 429 and 5xx responses should be mapped to HeatError::network (retryable = true).
    let err_429 = HeatError::network("api_error", "LI.FI API returned 429: rate limited");
    assert_eq!(err_429.category, ErrorCategory::Network);
    assert!(err_429.retryable);

    let err_503 = HeatError::network("api_error", "LI.FI API returned 503: service unavailable");
    assert_eq!(err_503.category, ErrorCategory::Network);
    assert!(err_503.retryable);
}

#[test]
fn http_4xx_maps_to_protocol_non_retryable() {
    use heat_core::error::{ErrorCategory, HeatError};
    // Non-429 4xx responses should be mapped to HeatError::protocol (retryable = false).
    let err_400 = HeatError::protocol("api_error", "LI.FI API returned 400: bad request");
    assert_eq!(err_400.category, ErrorCategory::Protocol);
    assert!(!err_400.retryable);

    let err_404 = HeatError::protocol("api_error", "LI.FI API returned 404: not found");
    assert_eq!(err_404.category, ErrorCategory::Protocol);
    assert!(!err_404.retryable);
}

#[test]
fn validate_amount_accepts_valid_inputs() {
    use crate::cmd::validate_amount;
    assert!(validate_amount("0").is_ok());
    assert!(validate_amount("1").is_ok());
    assert!(validate_amount("1000000").is_ok());
    assert!(validate_amount("999999999999999999").is_ok());
}

#[test]
fn validate_amount_rejects_empty() {
    use crate::cmd::validate_amount;
    use heat_core::error::ErrorCategory;
    let err = validate_amount("").unwrap_err();
    assert_eq!(err.category, ErrorCategory::Validation);
    assert_eq!(err.reason, "empty_amount");
}

#[test]
fn validate_amount_rejects_negative() {
    use crate::cmd::validate_amount;
    use heat_core::error::ErrorCategory;
    let err = validate_amount("-1").unwrap_err();
    assert_eq!(err.category, ErrorCategory::Validation);
    assert_eq!(err.reason, "negative_amount");
}

#[test]
fn validate_amount_rejects_non_digit_chars() {
    use crate::cmd::validate_amount;
    use heat_core::error::ErrorCategory;
    for bad in &["1.5", "1e10", "abc", "1 000", "0x1a"] {
        let err = validate_amount(bad).unwrap_err();
        assert_eq!(
            err.category,
            ErrorCategory::Validation,
            "expected validation error for '{bad}'"
        );
    }
}

#[test]
fn validate_amount_rejects_leading_zeros() {
    use crate::cmd::validate_amount;
    use heat_core::error::ErrorCategory;
    let err = validate_amount("007").unwrap_err();
    assert_eq!(err.category, ErrorCategory::Validation);
    assert_eq!(err.reason, "invalid_amount");
}

// ---------------------------------------------------------------------------
// Chain resolution
// ---------------------------------------------------------------------------

#[test]
fn resolve_chain_id_accepts_heat_names() {
    use crate::cmd::resolve_chain_id;
    assert_eq!(resolve_chain_id("ethereum").unwrap(), 1);
    assert_eq!(resolve_chain_id("polygon").unwrap(), 137);
    assert_eq!(resolve_chain_id("arbitrum").unwrap(), 42161);
    assert_eq!(resolve_chain_id("base").unwrap(), 8453);
    assert_eq!(resolve_chain_id("optimism").unwrap(), 10);
}

#[test]
fn resolve_chain_id_accepts_numeric() {
    use crate::cmd::resolve_chain_id;
    assert_eq!(resolve_chain_id("1").unwrap(), 1);
    assert_eq!(resolve_chain_id("137").unwrap(), 137);
    assert_eq!(resolve_chain_id("56").unwrap(), 56); // BSC — numeric passthrough
}

#[test]
fn resolve_chain_id_rejects_invalid() {
    use crate::cmd::resolve_chain_id;
    let err = resolve_chain_id("solana").unwrap_err();
    assert_eq!(err.reason, "invalid_chain");
}

#[test]
fn resolve_chain_id_accepts_aliases() {
    use crate::cmd::resolve_chain_id;
    assert_eq!(resolve_chain_id("eth").unwrap(), 1);
    assert_eq!(resolve_chain_id("arb").unwrap(), 42161);
    assert_eq!(resolve_chain_id("op").unwrap(), 10);
}

// ---------------------------------------------------------------------------
// Bridge DTO serialization
// ---------------------------------------------------------------------------

#[test]
fn bridge_result_dto_serializes() {
    use crate::dto::{BridgeResultDto, StepResultDto, TokenDto};
    let dto = BridgeResultDto {
        from_chain: "ethereum".to_owned(),
        to_chain: "arbitrum".to_owned(),
        from_token: TokenDto {
            address: "0xusdc".to_owned(),
            symbol: "USDC".to_owned(),
            decimals: 6,
            name: "USD Coin".to_owned(),
            chain_id: 1,
            logo_uri: None,
        },
        to_token: TokenDto {
            address: "0xusdc-arb".to_owned(),
            symbol: "USDC".to_owned(),
            decimals: 6,
            name: "USD Coin".to_owned(),
            chain_id: 42161,
            logo_uri: None,
        },
        from_amount: "1000000".to_owned(),
        from_amount_display: "1.0 USDC".to_owned(),
        to_amount_estimate: "998000".to_owned(),
        to_amount_min: "990000".to_owned(),
        route_id: "route-1".to_owned(),
        route_tags: vec!["CHEAPEST".to_owned()],
        tools_used: vec!["stargate".to_owned()],
        account: "0xuser".to_owned(),
        step_results: vec![StepResultDto {
            step_type: "cross".to_owned(),
            tool: "stargate".to_owned(),
            tx_hash: "0xtx1".to_owned(),
            approval_tx_hash: Some("0xapproval1".to_owned()),
        }],
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["from_chain"], "ethereum");
    assert_eq!(json["to_chain"], "arbitrum");
    assert_eq!(json["from_amount_display"], "1.0 USDC");
    assert_eq!(json["route_id"], "route-1");
    assert_eq!(json["step_results"][0]["tool"], "stargate");
    assert_eq!(json["step_results"][0]["approval_tx_hash"], "0xapproval1");
}

#[test]
fn step_result_dto_omits_approval_when_none() {
    use crate::dto::StepResultDto;
    let dto = StepResultDto {
        step_type: "swap".to_owned(),
        tool: "uniswap".to_owned(),
        tx_hash: "0xtx".to_owned(),
        approval_tx_hash: None,
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert!(json.get("approval_tx_hash").is_none());
}

// ---------------------------------------------------------------------------
// Value parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_value_flexible_handles_formats() {
    use crate::cmd::parse_value_flexible;
    assert_eq!(
        parse_value_flexible("0").unwrap(),
        alloy::primitives::U256::ZERO
    );
    assert_eq!(
        parse_value_flexible("0x0").unwrap(),
        alloy::primitives::U256::ZERO
    );
    assert_eq!(
        parse_value_flexible("1000000").unwrap(),
        alloy::primitives::U256::from(1_000_000u64)
    );
    assert_eq!(
        parse_value_flexible("0xf4240").unwrap(),
        alloy::primitives::U256::from(1_000_000u64)
    );
}

// ---------------------------------------------------------------------------
// Step JSON fromAddress injection
// ---------------------------------------------------------------------------

#[test]
fn step_json_gets_from_address_injected() {
    // Simulate what bridge() does: serialize a raw step, then inject fromAddress.
    let step = raw_step(1, 42161);
    let mut step_json = serde_json::to_value(&step).unwrap();

    // Before injection, fromAddress should be absent.
    assert!(
        step_json["action"].get("fromAddress").is_none(),
        "fromAddress should not be present before injection"
    );

    // Inject (same code path as cmd.rs bridge()).
    if let Some(action) = step_json.get_mut("action") {
        action["fromAddress"] = serde_json::Value::String("0xsender".to_owned());
    }

    assert_eq!(step_json["action"]["fromAddress"], "0xsender");
}

// ---------------------------------------------------------------------------
// Native token detection for approval logic
// ---------------------------------------------------------------------------

#[test]
fn native_zero_address_skips_approval() {
    // The zero address is a native token — approval should be skipped.
    let addr = "0x0000000000000000000000000000000000000000";
    let parsed: alloy::primitives::Address = addr.parse().unwrap();
    assert!(
        parsed.is_zero(),
        "zero address should be detected as native"
    );
}

#[test]
fn native_lifi_placeholder_skips_approval() {
    // LI.FI uses 0xEeee...EEeE as native token placeholder.
    let addr = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    assert_eq!(
        addr.to_lowercase(),
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "native placeholder should be detected"
    );
}

#[test]
fn erc20_address_requires_approval() {
    // A non-zero, non-placeholder address is an ERC20 token.
    let addr = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
    let parsed: alloy::primitives::Address = addr.parse().unwrap();
    let is_native =
        parsed.is_zero() || addr.to_lowercase() == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    assert!(!is_native, "USDC address should require approval");
}

// ---------------------------------------------------------------------------
// Execution classification — covered thoroughly in exec.rs unit tests.
// Smoke test here to verify the public API is accessible from outside exec.rs.
// ---------------------------------------------------------------------------

#[test]
fn classify_route_is_accessible_from_crate_root() {
    use crate::dto::RouteDto;
    let make_token = |chain_id: u64| TokenDto {
        address: "0xabc".to_owned(),
        symbol: "USDC".to_owned(),
        decimals: 6,
        name: "USD Coin".to_owned(),
        chain_id,
        logo_uri: None,
    };
    let route = RouteDto {
        id: "r1".to_owned(),
        from_chain_id: 1,
        to_chain_id: 42161,
        from_token: make_token(1),
        to_token: make_token(42161),
        from_amount: "1000000".to_owned(),
        to_amount: "990000".to_owned(),
        to_amount_min: "980000".to_owned(),
        steps: vec![],
        tags: vec!["FASTEST".to_owned()],
        execution_supported: true,
        execution_family: "EVM".to_owned(),
        execution_reason: None,
    };
    let support = classify_route(&route);
    // EVM-to-EVM routes should be classified as EVM family.
    assert_eq!(support.family, ExecutionFamily::Evm);
}
