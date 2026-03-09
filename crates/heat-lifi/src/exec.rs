/// Execution capability classification for LI.FI routes.
///
/// Classification is **best-effort**. It indicates whether Heat has the
/// substrate infrastructure to execute a given route, not whether the route
/// will succeed on-chain.
///
/// ## How classification works
///
/// 1. **Preferred**: chain type metadata from the LI.FI `/chains` endpoint
///    (via `classify_route_with_chain_types`). This is authoritative for every
///    chain LI.FI supports.
/// 2. **Fallback**: a static chain-ID table in `family_from_chain_id`. Used
///    only when chain metadata is unavailable. This table is intentionally
///    conservative — unknown chain IDs map to `Unsupported` rather than
///    defaulting to EVM.
///
/// ## Capability matrix
///
/// | From → To                  | Status                             |
/// |----------------------------|------------------------------------|
/// | Heat EVM → Heat EVM        | Supported via heat-evm             |
/// | non-Heat EVM → *           | Not supported (no RPC/wallet)      |
/// | * → non-Heat EVM           | Not supported (no RPC/wallet)      |
/// | SVM → *                    | Not yet supported                  |
/// | * → SVM                    | Not yet supported                  |
/// | Other                      | Not yet supported                  |
///
/// Heat-supported EVM chains: ethereum, polygon, arbitrum, optimism, base.
use crate::dto::RouteDto;

// ---------------------------------------------------------------------------
// Chain family classification
// ---------------------------------------------------------------------------

/// Broad chain execution family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionFamily {
    /// Standard EVM chains (Ethereum, Arbitrum, Optimism, Polygon, Base, …).
    Evm,
    /// Solana.
    Solana,
    /// Any other chain family not yet modelled.
    Unsupported(String),
}

impl ExecutionFamily {
    /// Classify a LI.FI chain type string into an `ExecutionFamily`.
    pub fn from_chain_type(chain_type: &str) -> Self {
        match chain_type.to_uppercase().as_str() {
            "EVM" => Self::Evm,
            "SVM" | "SOLANA" => Self::Solana,
            other => Self::Unsupported(other.to_owned()),
        }
    }
}

impl std::fmt::Display for ExecutionFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Evm => write!(f, "EVM"),
            Self::Solana => write!(f, "Solana"),
            Self::Unsupported(s) => write!(f, "unsupported({s})"),
        }
    }
}

// ---------------------------------------------------------------------------
// Execution support result
// ---------------------------------------------------------------------------

/// Result of classifying whether Heat can execute a given route.
#[derive(Debug, Clone)]
pub struct ExecutionSupport {
    /// `true` if Heat has the execution capability for this route today.
    pub supported: bool,
    /// The chain family driving the classification.
    pub family: ExecutionFamily,
    /// Human-readable reason when `supported` is `false`.
    pub reason: Option<String>,
}

impl ExecutionSupport {
    /// Construct a positive (supported) classification.
    #[allow(dead_code)]
    pub fn yes(family: ExecutionFamily) -> Self {
        Self {
            supported: true,
            family,
            reason: None,
        }
    }

    fn no(family: ExecutionFamily, reason: impl Into<String>) -> Self {
        Self {
            supported: false,
            family,
            reason: Some(reason.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Route classification
// ---------------------------------------------------------------------------

/// Determine whether Heat can execute `route` today.
///
/// A route is executable if:
/// 1. Both the source and destination chains are **Heat-supported** EVM chains
///    (i.e. present in `heat_evm::EvmChain`).
/// 2. Every intermediate step also uses Heat-supported EVM chains.
///
/// This is intentionally tighter than "all chains are EVM". Heat can only
/// execute transactions on chains it has RPC configuration and wallet support
/// for, so an EVM chain outside the Heat-supported set is classified as
/// unsupported for execution.
///
/// Current execution capability matrix:
/// - Heat EVM → Heat EVM: supported via heat-evm
/// - non-Heat EVM:        not yet supported (no RPC/wallet config)
/// - anything → SVM:      not yet supported
/// - SVM → anything:      not yet supported
/// - other:               not yet supported
pub fn classify_route(route: &RouteDto) -> ExecutionSupport {
    let from_family = family_from_chain_id(route.from_chain_id);
    let to_family = family_from_chain_id(route.to_chain_id);

    // Reject if either endpoint is not EVM.
    if from_family != ExecutionFamily::Evm {
        return ExecutionSupport::no(
            from_family.clone(),
            format!("Source chain {from_family} execution not yet supported"),
        );
    }
    if to_family != ExecutionFamily::Evm {
        return ExecutionSupport::no(
            to_family.clone(),
            format!("Destination chain {to_family} execution not yet supported"),
        );
    }

    // Reject if either endpoint is EVM but not Heat-supported.
    if !is_heat_supported_chain(route.from_chain_id) {
        return ExecutionSupport::no(
            ExecutionFamily::Evm,
            format!(
                "Source chain {} is EVM but not yet supported by Heat",
                route.from_chain_id
            ),
        );
    }
    if !is_heat_supported_chain(route.to_chain_id) {
        return ExecutionSupport::no(
            ExecutionFamily::Evm,
            format!(
                "Destination chain {} is EVM but not yet supported by Heat",
                route.to_chain_id
            ),
        );
    }

    // Check intermediate steps.
    for step in &route.steps {
        let step_from = family_from_chain_id(step.from_token.chain_id);
        let step_to = family_from_chain_id(step.to_token.chain_id);
        if step_from != ExecutionFamily::Evm {
            return ExecutionSupport::no(
                step_from.clone(),
                format!("Step uses {step_from} source chain — not yet supported"),
            );
        }
        if step_to != ExecutionFamily::Evm {
            return ExecutionSupport::no(
                step_to.clone(),
                format!("Step uses {step_to} destination chain — not yet supported"),
            );
        }
        if !is_heat_supported_chain(step.from_token.chain_id) {
            return ExecutionSupport::no(
                ExecutionFamily::Evm,
                format!(
                    "Step uses chain {} which is not yet supported by Heat",
                    step.from_token.chain_id
                ),
            );
        }
        if !is_heat_supported_chain(step.to_token.chain_id) {
            return ExecutionSupport::no(
                ExecutionFamily::Evm,
                format!(
                    "Step uses chain {} which is not yet supported by Heat",
                    step.to_token.chain_id
                ),
            );
        }
    }

    // All chains are Heat-supported EVM — executable via heat-evm.
    ExecutionSupport::yes(ExecutionFamily::Evm)
}

/// Classify a route using chain type metadata from the LI.FI `/chains` response.
///
/// When `chain_types` contains an entry for a given chain ID, that entry drives
/// family classification. Remaining chain IDs fall back to `family_from_chain_id()`.
///
/// Execution support additionally requires all chains to be Heat-supported
/// (present in `heat_evm::EvmChain`).
pub fn classify_route_with_chain_types(
    route: &RouteDto,
    chain_types: &std::collections::HashMap<u64, String>,
) -> ExecutionSupport {
    let from_family = chain_types
        .get(&route.from_chain_id)
        .map(|ct| ExecutionFamily::from_chain_type(ct))
        .unwrap_or_else(|| family_from_chain_id(route.from_chain_id));
    let to_family = chain_types
        .get(&route.to_chain_id)
        .map(|ct| ExecutionFamily::from_chain_type(ct))
        .unwrap_or_else(|| family_from_chain_id(route.to_chain_id));

    if from_family != ExecutionFamily::Evm {
        return ExecutionSupport::no(
            from_family.clone(),
            format!("Source chain {from_family} execution not yet supported"),
        );
    }
    if to_family != ExecutionFamily::Evm {
        return ExecutionSupport::no(
            to_family.clone(),
            format!("Destination chain {to_family} execution not yet supported"),
        );
    }

    // EVM but not Heat-supported.
    if !is_heat_supported_chain(route.from_chain_id) {
        return ExecutionSupport::no(
            ExecutionFamily::Evm,
            format!(
                "Source chain {} is EVM but not yet supported by Heat",
                route.from_chain_id
            ),
        );
    }
    if !is_heat_supported_chain(route.to_chain_id) {
        return ExecutionSupport::no(
            ExecutionFamily::Evm,
            format!(
                "Destination chain {} is EVM but not yet supported by Heat",
                route.to_chain_id
            ),
        );
    }

    for step in &route.steps {
        let step_from = chain_types
            .get(&step.from_token.chain_id)
            .map(|ct| ExecutionFamily::from_chain_type(ct))
            .unwrap_or_else(|| family_from_chain_id(step.from_token.chain_id));
        let step_to = chain_types
            .get(&step.to_token.chain_id)
            .map(|ct| ExecutionFamily::from_chain_type(ct))
            .unwrap_or_else(|| family_from_chain_id(step.to_token.chain_id));
        if step_from != ExecutionFamily::Evm {
            return ExecutionSupport::no(
                step_from.clone(),
                format!("Step uses {step_from} source chain — not yet supported"),
            );
        }
        if step_to != ExecutionFamily::Evm {
            return ExecutionSupport::no(
                step_to.clone(),
                format!("Step uses {step_to} destination chain — not yet supported"),
            );
        }
        if !is_heat_supported_chain(step.from_token.chain_id) {
            return ExecutionSupport::no(
                ExecutionFamily::Evm,
                format!(
                    "Step uses chain {} which is not yet supported by Heat",
                    step.from_token.chain_id
                ),
            );
        }
        if !is_heat_supported_chain(step.to_token.chain_id) {
            return ExecutionSupport::no(
                ExecutionFamily::Evm,
                format!(
                    "Step uses chain {} which is not yet supported by Heat",
                    step.to_token.chain_id
                ),
            );
        }
    }

    ExecutionSupport::yes(ExecutionFamily::Evm)
}

// ---------------------------------------------------------------------------
// Heat-supported chain check
// ---------------------------------------------------------------------------

/// Returns `true` if `chain_id` corresponds to a chain in `heat_evm::EvmChain`.
///
/// This is the gate for `execution_supported = true`. Heat can only execute
/// transactions on chains it has RPC defaults and wallet infrastructure for.
fn is_heat_supported_chain(chain_id: u64) -> bool {
    heat_evm::EvmChain::from_chain_id(chain_id).is_some()
}

// ---------------------------------------------------------------------------
// Chain ID → family heuristic
// ---------------------------------------------------------------------------

/// Static fallback: map a chain ID to an execution family using well-known chain IDs.
///
/// This is a **fallback heuristic** used only when chain type metadata from
/// the `/chains` API is unavailable. Prefer `classify_route_with_chain_types`
/// which uses authoritative chain metadata.
///
/// Intentionally conservative: unknown IDs return `Unsupported` rather than
/// assuming EVM.
pub fn family_from_chain_id(chain_id: u64) -> ExecutionFamily {
    match chain_id {
        // Solana mainnet magic number used by LI.FI
        1151111081099710 => ExecutionFamily::Solana,
        // Well-known EVM chains
        1 | 10 | 56 | 100 | 137 | 250 | 324 | 1101 | 8453 | 42161 | 42220 | 43114 | 59144
        | 534352 => ExecutionFamily::Evm,
        // Unknown — do not assume EVM
        other => ExecutionFamily::Unsupported(format!("chain-{other}")),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{EstimateDto, RouteDto, StepDto, TokenDto};

    fn evm_token(chain_id: u64) -> TokenDto {
        TokenDto {
            address: "0xabc".to_owned(),
            symbol: "USDC".to_owned(),
            decimals: 6,
            name: "USD Coin".to_owned(),
            chain_id,
            logo_uri: None,
        }
    }

    fn empty_estimate() -> EstimateDto {
        EstimateDto {
            from_amount: "1000000".to_owned(),
            to_amount: "990000".to_owned(),
            to_amount_min: "980000".to_owned(),
            execution_duration: 30,
            fees: vec![],
        }
    }

    fn evm_step(from_chain: u64, to_chain: u64) -> StepDto {
        StepDto {
            step_type: "cross".to_owned(),
            tool: "stargate".to_owned(),
            from_token: evm_token(from_chain),
            to_token: evm_token(to_chain),
            from_amount: "1000000".to_owned(),
            to_amount: "990000".to_owned(),
            estimate: empty_estimate(),
        }
    }

    fn evm_route(from_chain: u64, to_chain: u64) -> RouteDto {
        RouteDto {
            id: "test-route".to_owned(),
            from_chain_id: from_chain,
            to_chain_id: to_chain,
            from_token: evm_token(from_chain),
            to_token: evm_token(to_chain),
            from_amount: "1000000".to_owned(),
            to_amount: "990000".to_owned(),
            to_amount_min: "980000".to_owned(),
            steps: vec![evm_step(from_chain, to_chain)],
            tags: vec!["CHEAPEST".to_owned()],
            execution_supported: false,
            execution_family: String::new(),
            execution_reason: None,
        }
    }

    #[test]
    fn evm_to_evm_route_classified_as_evm_family() {
        // Ethereum mainnet (1) -> Arbitrum (42161)
        let route = evm_route(1, 42161);
        let support = classify_route(&route);
        assert_eq!(support.family, ExecutionFamily::Evm);
    }

    #[test]
    fn evm_to_evm_route_is_supported() {
        let route = evm_route(1, 42161);
        let support = classify_route(&route);
        assert!(support.supported);
        assert!(support.reason.is_none());
    }

    #[test]
    fn solana_source_is_unsupported() {
        let mut route = evm_route(1, 42161);
        route.from_chain_id = 1151111081099710; // Solana chain ID
        let support = classify_route(&route);
        assert!(!support.supported);
        assert_eq!(support.family, ExecutionFamily::Solana);
    }

    #[test]
    fn solana_destination_is_unsupported() {
        let mut route = evm_route(1, 42161);
        route.to_chain_id = 1151111081099710;
        let support = classify_route(&route);
        assert!(!support.supported);
    }

    #[test]
    fn execution_family_display() {
        assert_eq!(ExecutionFamily::Evm.to_string(), "EVM");
        assert_eq!(ExecutionFamily::Solana.to_string(), "Solana");
        assert_eq!(
            ExecutionFamily::Unsupported("COSMOS".to_owned()).to_string(),
            "unsupported(COSMOS)"
        );
    }

    #[test]
    fn chain_type_metadata_overrides_static_list_for_family() {
        // chain ID 999999 is not in the static allow-list and would normally
        // resolve to Unsupported, but the chain_types map marks it as EVM.
        // Family is EVM, but execution_supported is false because 999999 is
        // not a Heat-supported chain.
        let mut chain_types = std::collections::HashMap::new();
        chain_types.insert(999999u64, "EVM".to_string());
        chain_types.insert(1u64, "EVM".to_string());
        let route = evm_route(999999, 1);
        let support = classify_route_with_chain_types(&route, &chain_types);
        assert_eq!(support.family, ExecutionFamily::Evm);
        assert!(
            !support.supported,
            "non-Heat EVM chain should not be marked supported"
        );
    }

    #[test]
    fn chain_type_metadata_svm_is_unsupported() {
        let mut chain_types = std::collections::HashMap::new();
        chain_types.insert(1151111081099710u64, "SVM".to_string());
        chain_types.insert(1u64, "EVM".to_string());
        let mut route = evm_route(1, 42161);
        route.from_chain_id = 1151111081099710;
        let support = classify_route_with_chain_types(&route, &chain_types);
        assert!(!support.supported);
        assert_eq!(support.family, ExecutionFamily::Solana);
    }

    #[test]
    fn non_heat_evm_chain_is_unsupported_for_execution() {
        // BSC (chain 56) is EVM but not a Heat-supported chain.
        let route = evm_route(56, 1);
        let support = classify_route(&route);
        assert_eq!(support.family, ExecutionFamily::Evm);
        assert!(!support.supported);
        assert!(
            support
                .reason
                .as_ref()
                .unwrap()
                .contains("not yet supported by Heat")
        );
    }

    #[test]
    fn non_heat_evm_destination_is_unsupported() {
        // Ethereum → BSC: source is Heat-supported but dest is not.
        let route = evm_route(1, 56);
        let support = classify_route(&route);
        assert!(!support.supported);
        assert!(
            support
                .reason
                .as_ref()
                .unwrap()
                .contains("not yet supported by Heat")
        );
    }

    #[test]
    fn heat_supported_evm_route_is_supported() {
        // All Heat chains: 1 (eth), 137 (polygon), 42161 (arb), 10 (op), 8453 (base)
        for (from, to) in &[(1, 137), (42161, 8453), (10, 1)] {
            let route = evm_route(*from, *to);
            let support = classify_route(&route);
            assert!(support.supported, "route {from} → {to} should be supported");
        }
    }

    #[test]
    fn step_with_non_heat_chain_is_unsupported() {
        // Route endpoints are Heat-supported, but step uses a non-Heat chain.
        let mut route = evm_route(1, 42161);
        route.steps = vec![evm_step(1, 56)]; // step goes to BSC
        let support = classify_route(&route);
        assert!(!support.supported);
    }

    #[test]
    fn family_from_chain_type_mapping() {
        assert_eq!(
            ExecutionFamily::from_chain_type("EVM"),
            ExecutionFamily::Evm
        );
        assert_eq!(
            ExecutionFamily::from_chain_type("SVM"),
            ExecutionFamily::Solana
        );
        assert_eq!(
            ExecutionFamily::from_chain_type("SOLANA"),
            ExecutionFamily::Solana
        );
        assert!(matches!(
            ExecutionFamily::from_chain_type("COSMOS"),
            ExecutionFamily::Unsupported(_)
        ));
    }
}
