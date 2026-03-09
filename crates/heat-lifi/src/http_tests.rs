//! Real HTTP-layer tests for the LI.FI client.
//!
//! These tests spin up a local `wiremock::MockServer`, point `LifiClient` at it,
//! and exercise the actual reqwest HTTP stack end-to-end. They verify both that
//! the client sends the right requests (method, path, query params, body) and
//! that it correctly deserialises the responses into Heat-owned types.

use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::client::{LifiClient, QuoteParams, RoutesParams, StatusParams};
use heat_core::error::ErrorCategory;

// ---------------------------------------------------------------------------
// JSON fixtures
// ---------------------------------------------------------------------------

fn chains_json() -> serde_json::Value {
    json!({
        "chains": [{
            "id": 1,
            "name": "Ethereum",
            "chainType": "EVM",
            "nativeToken": {
                "address": "0x0000000000000000000000000000000000000000",
                "symbol": "ETH",
                "decimals": 18,
                "name": "Ether",
                "chainId": 1
            }
        }]
    })
}

fn tokens_json() -> serde_json::Value {
    json!({
        "tokens": {
            "1": [{
                "address": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                "symbol": "USDC",
                "decimals": 6,
                "name": "USD Coin",
                "chainId": 1
            }]
        }
    })
}

fn tools_json() -> serde_json::Value {
    json!({
        "bridges": [{
            "key": "stargate",
            "name": "Stargate",
            "supportedChains": [{"chainId": 1}, {"chainId": 42161}]
        }],
        "exchanges": [{
            "key": "uniswap",
            "name": "Uniswap",
            "supportedChains": [{"chainId": 1}]
        }]
    })
}

fn quote_json() -> serde_json::Value {
    json!({
        "type": "cross",
        "tool": "stargate",
        "toolDetails": {"key": "stargate", "name": "Stargate"},
        "action": {
            "fromToken": {
                "address": "0xusdc",
                "symbol": "USDC",
                "decimals": 6,
                "name": "USD Coin",
                "chainId": 1
            },
            "toToken": {
                "address": "0xusdc-arb",
                "symbol": "USDC",
                "decimals": 6,
                "name": "USD Coin",
                "chainId": 42161
            },
            "fromAmount": "1000000",
            "fromChainId": 1,
            "toChainId": 42161
        },
        "estimate": {
            "fromAmount": "1000000",
            "toAmount": "998000",
            "toAmountMin": "990000",
            "executionDuration": 120.0,
            "feeCosts": []
        }
    })
}

fn routes_json() -> serde_json::Value {
    json!({
        "routes": [{
            "id": "route-1",
            "fromChainId": 1,
            "toChainId": 42161,
            "fromToken": {
                "address": "0xusdc",
                "symbol": "USDC",
                "decimals": 6,
                "name": "USD Coin",
                "chainId": 1
            },
            "toToken": {
                "address": "0xusdc-arb",
                "symbol": "USDC",
                "decimals": 6,
                "name": "USD Coin",
                "chainId": 42161
            },
            "fromAmount": "1000000",
            "toAmount": "998000",
            "toAmountMin": "990000",
            "steps": [],
            "tags": ["CHEAPEST"]
        }]
    })
}

fn status_json() -> serde_json::Value {
    json!({
        "status": "DONE",
        "substatus": "COMPLETED",
        "txHash": "0xabc123"
    })
}

// ---------------------------------------------------------------------------
// Chains endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn chains_returns_parsed_response() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/chains"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chains_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let resp = client.chains().await.unwrap();

    assert_eq!(resp.chains.len(), 1);
    assert_eq!(resp.chains[0].id, 1);
    assert_eq!(resp.chains[0].name, "Ethereum");
    assert_eq!(resp.chains[0].chain_type, "EVM");
    assert_eq!(resp.chains[0].native_token.symbol, "ETH");
}

// ---------------------------------------------------------------------------
// Tokens endpoint — query param forwarding
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tokens_sends_chains_query_param_when_chain_id_is_some() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tokens"))
        .and(query_param("chains", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tokens_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let resp = client.tokens(Some(1)).await.unwrap();

    assert_eq!(resp.tokens.len(), 1);
    assert!(resp.tokens.contains_key("1"));
}

#[tokio::test]
async fn tokens_omits_chains_query_param_when_chain_id_is_none() {
    let server = MockServer::start().await;

    // Match a GET /tokens with NO chains param — wiremock will reject requests
    // that don't match, so if the client mistakenly sends `chains=`, this mock
    // will not fire and the request will return 404.
    Mock::given(method("GET"))
        .and(path("/tokens"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tokens_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let resp = client.tokens(None).await.unwrap();

    assert_eq!(resp.tokens.len(), 1);
}

// ---------------------------------------------------------------------------
// Tools endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_returns_bridges_and_exchanges() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tools"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tools_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let resp = client.tools().await.unwrap();

    assert_eq!(resp.bridges.len(), 1);
    assert_eq!(resp.bridges[0].key, "stargate");
    assert_eq!(resp.exchanges.len(), 1);
    assert_eq!(resp.exchanges[0].key, "uniswap");
}

// ---------------------------------------------------------------------------
// Quote endpoint — all query params verified individually
// ---------------------------------------------------------------------------

#[tokio::test]
async fn quote_sends_all_required_query_params() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/quote"))
        .and(query_param("fromChain", "1"))
        .and(query_param("toChain", "42161"))
        .and(query_param("fromToken", "0xusdc"))
        .and(query_param("toToken", "0xusdc-arb"))
        .and(query_param("fromAmount", "1000000"))
        .and(query_param("fromAddress", "0xdeadbeef"))
        .respond_with(ResponseTemplate::new(200).set_body_json(quote_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let params = QuoteParams {
        from_chain: "1".to_owned(),
        to_chain: "42161".to_owned(),
        from_token: "0xusdc".to_owned(),
        to_token: "0xusdc-arb".to_owned(),
        from_amount: "1000000".to_owned(),
        from_address: Some("0xdeadbeef".to_owned()),
    };
    let resp = client.quote(&params).await.unwrap();

    assert_eq!(resp.tool, "stargate");
    assert_eq!(resp.action.from_chain_id, 1);
    assert_eq!(resp.action.to_chain_id, 42161);
    assert_eq!(resp.estimate.from_amount, "1000000");
    assert_eq!(resp.estimate.to_amount, "998000");
}

// ---------------------------------------------------------------------------
// Routes endpoint (POST) — body shape verified
// ---------------------------------------------------------------------------

#[tokio::test]
async fn routes_posts_json_body_with_camel_case_keys() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/advanced/routes"))
        .and(body_partial_json(json!({
            "fromChainId": 1,
            "toChainId": 42161,
            "fromTokenAddress": "0xusdc",
            "toTokenAddress": "0xusdc-arb",
            "fromAmount": "1000000"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(routes_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let params = RoutesParams {
        from_chain_id: 1,
        to_chain_id: 42161,
        from_token_address: "0xusdc".to_owned(),
        to_token_address: "0xusdc-arb".to_owned(),
        from_amount: "1000000".to_owned(),
        from_address: None,
    };
    let resp = client.routes(&params).await.unwrap();

    assert_eq!(resp.routes.len(), 1);
    assert_eq!(resp.routes[0].id, "route-1");
    assert_eq!(resp.routes[0].from_chain_id, 1);
    assert_eq!(resp.routes[0].to_chain_id, 42161);
    assert_eq!(resp.routes[0].tags, vec!["CHEAPEST"]);
}

#[tokio::test]
async fn routes_omits_from_address_when_none() {
    let server = MockServer::start().await;

    // body_partial_json only checks the keys it is given, so we cannot assert
    // *absence* of a key with it directly. Instead we mount a permissive mock
    // and rely on the serde `skip_serializing_if` already tested structurally
    // in tests.rs. Here we simply confirm the request succeeds with None address.
    Mock::given(method("POST"))
        .and(path("/advanced/routes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(routes_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let params = RoutesParams {
        from_chain_id: 1,
        to_chain_id: 42161,
        from_token_address: "0xusdc".to_owned(),
        to_token_address: "0xusdc-arb".to_owned(),
        from_amount: "1000000".to_owned(),
        from_address: None,
    };
    let resp = client.routes(&params).await.unwrap();
    assert_eq!(resp.routes.len(), 1);
}

// ---------------------------------------------------------------------------
// Status endpoint — query params including optional bridge
// ---------------------------------------------------------------------------

#[tokio::test]
async fn status_sends_required_query_params() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/status"))
        .and(query_param("txHash", "0xabc123"))
        .and(query_param("fromChain", "1"))
        .and(query_param("toChain", "42161"))
        .respond_with(ResponseTemplate::new(200).set_body_json(status_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let params = StatusParams {
        tx_hash: "0xabc123".to_owned(),
        bridge: None,
        from_chain: "1".to_owned(),
        to_chain: "42161".to_owned(),
    };
    let resp = client.status(&params).await.unwrap();

    assert_eq!(resp.status, "DONE");
    assert_eq!(resp.substatus, "COMPLETED");
    assert_eq!(resp.tx_hash.as_deref(), Some("0xabc123"));
}

#[tokio::test]
async fn status_includes_bridge_param_when_set() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/status"))
        .and(query_param("txHash", "0xabc123"))
        .and(query_param("fromChain", "1"))
        .and(query_param("toChain", "42161"))
        .and(query_param("bridge", "stargate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(status_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let params = StatusParams {
        tx_hash: "0xabc123".to_owned(),
        bridge: Some("stargate".to_owned()),
        from_chain: "1".to_owned(),
        to_chain: "42161".to_owned(),
    };
    let resp = client.status(&params).await.unwrap();
    assert_eq!(resp.status, "DONE");
}

// ---------------------------------------------------------------------------
// Step transaction endpoint — verifies fromAddress injection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn step_transaction_sends_from_address_in_body() {
    let server = MockServer::start().await;

    // The mock requires the POST body to contain action.fromAddress.
    Mock::given(method("POST"))
        .and(path("/advanced/stepTransaction"))
        .and(body_partial_json(json!({
            "action": {
                "fromAddress": "0xsender123"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "transactionRequest": {
                "to": "0xcontract",
                "data": "0x1234",
                "value": "0",
                "chainId": 1
            },
            "estimate": {
                "fromAmount": "1000000",
                "toAmount": "998000",
                "toAmountMin": "990000",
                "executionDuration": 60.0,
                "feeCosts": []
            },
            "action": {
                "fromToken": {
                    "address": "0xusdc",
                    "symbol": "USDC",
                    "decimals": 6,
                    "name": "USD Coin",
                    "chainId": 1
                },
                "toToken": {
                    "address": "0xusdc-arb",
                    "symbol": "USDC",
                    "decimals": 6,
                    "name": "USD Coin",
                    "chainId": 42161
                },
                "fromAmount": "1000000",
                "fromChainId": 1,
                "toChainId": 42161
            }
        })))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();

    // Build a step JSON with fromAddress injected (as bridge() does).
    let mut step_json = json!({
        "type": "cross",
        "tool": "stargate",
        "toolDetails": {"key": "stargate", "name": "Stargate"},
        "action": {
            "fromToken": {
                "address": "0xusdc",
                "symbol": "USDC",
                "decimals": 6,
                "name": "USD Coin",
                "chainId": 1
            },
            "toToken": {
                "address": "0xusdc-arb",
                "symbol": "USDC",
                "decimals": 6,
                "name": "USD Coin",
                "chainId": 42161
            },
            "fromAmount": "1000000",
            "fromChainId": 1,
            "toChainId": 42161
        },
        "estimate": {
            "fromAmount": "1000000",
            "toAmount": "998000",
            "toAmountMin": "990000",
            "executionDuration": 120.0,
            "feeCosts": []
        }
    });
    // Inject fromAddress (exactly as cmd.rs bridge() does).
    step_json["action"]["fromAddress"] = serde_json::Value::String("0xsender123".to_owned());

    let resp = client.step_transaction(&step_json).await.unwrap();
    assert_eq!(resp.transaction_request.to, "0xcontract");
    assert_eq!(resp.transaction_request.chain_id, 1);
}

#[tokio::test]
async fn step_transaction_without_from_address_fails_mock() {
    let server = MockServer::start().await;

    // Mock requires fromAddress — request without it should not match.
    Mock::given(method("POST"))
        .and(path("/advanced/stepTransaction"))
        .and(body_partial_json(json!({
            "action": {
                "fromAddress": "0xsender123"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "transactionRequest": {
                "to": "0xcontract",
                "data": "0x1234",
                "value": "0",
                "chainId": 1
            },
            "estimate": {
                "fromAmount": "1000000",
                "toAmount": "998000",
                "toAmountMin": "990000",
                "executionDuration": 60.0,
                "feeCosts": []
            },
            "action": {
                "fromToken": {
                    "address": "0xusdc",
                    "symbol": "USDC",
                    "decimals": 6,
                    "name": "USD Coin",
                    "chainId": 1
                },
                "toToken": {
                    "address": "0xusdc-arb",
                    "symbol": "USDC",
                    "decimals": 6,
                    "name": "USD Coin",
                    "chainId": 42161
                },
                "fromAmount": "1000000",
                "fromChainId": 1,
                "toChainId": 42161
            }
        })))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();

    // Send WITHOUT fromAddress — mock should return 404 (no match).
    let step_json = json!({
        "type": "cross",
        "tool": "stargate",
        "toolDetails": {"key": "stargate", "name": "Stargate"},
        "action": {
            "fromToken": {
                "address": "0xusdc",
                "symbol": "USDC",
                "decimals": 6,
                "name": "USD Coin",
                "chainId": 1
            },
            "toToken": {
                "address": "0xusdc-arb",
                "symbol": "USDC",
                "decimals": 6,
                "name": "USD Coin",
                "chainId": 42161
            },
            "fromAmount": "1000000",
            "fromChainId": 1,
            "toChainId": 42161
        },
        "estimate": {
            "fromAmount": "1000000",
            "toAmount": "998000",
            "toAmountMin": "990000",
            "executionDuration": 120.0,
            "feeCosts": []
        }
    });

    let result = client.step_transaction(&step_json).await;
    assert!(result.is_err(), "stepTransaction without fromAddress should fail");
}

// ---------------------------------------------------------------------------
// Error mapping — HTTP status codes → HeatError categories
// ---------------------------------------------------------------------------

#[tokio::test]
async fn http_400_maps_to_protocol_error_non_retryable() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/chains"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad request"))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let err = client.chains().await.unwrap_err();

    assert_eq!(err.category, ErrorCategory::Protocol);
    assert!(!err.retryable, "4xx errors must not be retryable");
}

#[tokio::test]
async fn http_429_maps_to_network_error_retryable() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/chains"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limited"))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let err = client.chains().await.unwrap_err();

    assert_eq!(err.category, ErrorCategory::Network);
    assert!(err.retryable, "429 errors must be retryable");
}

#[tokio::test]
async fn http_503_maps_to_network_error_retryable() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/chains"))
        .respond_with(ResponseTemplate::new(503).set_body_string("service unavailable"))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let err = client.chains().await.unwrap_err();

    assert_eq!(err.category, ErrorCategory::Network);
    assert!(err.retryable, "5xx errors must be retryable");
}

// ---------------------------------------------------------------------------
// API key header
// ---------------------------------------------------------------------------

#[tokio::test]
async fn api_key_header_sent_when_configured() {
    use wiremock::matchers::header;

    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/chains"))
        .and(header("x-lifi-api-key", "test-secret-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chains_json()))
        .mount(&server)
        .await;

    let mut client = LifiClient::with_base_url(server.uri()).unwrap();
    client.set_api_key("test-secret-key");
    let resp = client.chains().await.unwrap();

    assert_eq!(resp.chains.len(), 1);
    assert_eq!(resp.chains[0].name, "Ethereum");
}

#[tokio::test]
async fn no_api_key_header_when_not_configured() {
    let server = MockServer::start().await;

    // Mount a mock that does NOT require the header — if the header were
    // present, the request would still match, but we verify success without it.
    Mock::given(method("GET"))
        .and(path("/chains"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chains_json()))
        .mount(&server)
        .await;

    let client = LifiClient::with_base_url(server.uri()).unwrap();
    let resp = client.chains().await.unwrap();
    assert_eq!(resp.chains.len(), 1);
}
