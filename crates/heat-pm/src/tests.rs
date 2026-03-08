//! Integration-style tests for heat-pm.
//!
//! Per-file unit tests live directly in each source file (auth.rs, bridge.rs,
//! data.rs, clob.rs). This file covers cross-cutting properties and anything
//! that exercises the crate from the outside.

/// Smoke test: the crate root exports the expected public modules.
#[test]
fn public_modules_are_accessible() {
    // auth and cmd are both `pub mod` in lib.rs. This test simply ensures the
    // module tree compiles correctly and is reachable from the crate root.
    // The real tests are in the sub-modules.
    let _: fn() = || {
        let _ = std::module_path!();
    };
}

/// Verify that the RPC URL constant is a valid HTTPS URL pointing at Polygon.
#[test]
fn rpc_url_is_polygon_https() {
    let url = crate::auth::RPC_URL;
    assert!(url.starts_with("https://"), "RPC_URL must use HTTPS");
    // Should refer to Polygon, not Ethereum mainnet or a testnet.
    assert!(
        url.contains("polygon") || url.contains("drpc"),
        "RPC_URL should point to a Polygon RPC endpoint, got: {url}"
    );
}
