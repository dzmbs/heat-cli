/// Mapping layer: raw LI.FI API responses → Heat-owned DTOs.
///
/// Nothing in this module is part of Heat's public contract. Only the
/// output DTOs from `dto.rs` cross the boundary into commands / tests.
use crate::client::{
    ChainsResponse, QuoteResponse, RawBridgeTool, RawChain, RawEstimate, RawExchangeTool, RawRoute,
    RawStep, RawToken, RoutesResponse, StatusResponse, TokensResponse, ToolsResponse,
};
use crate::dto::{
    BridgeToolDto, ChainDto, ChainsListDto, EstimateDto, ExchangeToolDto, FeeDto, QuoteDto,
    RouteDto, RoutesListDto, StatusDto, StepDto, TokenDto, TokensListDto, ToolDetailsDto, ToolsDto,
};
use crate::exec::classify_route_with_chain_types;

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

pub fn map_token(raw: &RawToken) -> TokenDto {
    TokenDto {
        address: raw.address.clone(),
        symbol: raw.symbol.clone(),
        decimals: raw.decimals,
        name: raw.name.clone(),
        chain_id: raw.chain_id,
        logo_uri: raw.logo_uri.clone(),
    }
}

// ---------------------------------------------------------------------------
// Chain
// ---------------------------------------------------------------------------

fn map_chain(raw: &RawChain) -> ChainDto {
    ChainDto {
        id: raw.id,
        name: raw.name.clone(),
        chain_type: raw.chain_type.clone(),
        native_token: map_token(&raw.native_token),
    }
}

pub fn map_chains(resp: ChainsResponse) -> ChainsListDto {
    ChainsListDto {
        chains: resp.chains.iter().map(map_chain).collect(),
    }
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

pub fn map_tokens(resp: TokensResponse, chain_id: Option<u64>) -> TokensListDto {
    let mut tokens: Vec<TokenDto> = resp
        .tokens
        .values()
        .flat_map(|list| list.iter().map(map_token))
        .collect();
    tokens.sort_by(|a, b| {
        a.chain_id
            .cmp(&b.chain_id)
            .then_with(|| a.symbol.cmp(&b.symbol))
            .then_with(|| a.address.cmp(&b.address))
    });
    TokensListDto { tokens, chain_id }
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

fn map_bridge_tool(raw: &RawBridgeTool) -> BridgeToolDto {
    BridgeToolDto {
        key: raw.key.clone(),
        name: raw.name.clone(),
        logo_uri: raw.logo_uri.clone(),
        supported_chains: raw.supported_chains.iter().map(|c| c.chain_id).collect(),
    }
}

fn map_exchange_tool(raw: &RawExchangeTool) -> ExchangeToolDto {
    ExchangeToolDto {
        key: raw.key.clone(),
        name: raw.name.clone(),
        logo_uri: raw.logo_uri.clone(),
        supported_chains: raw.supported_chains.iter().map(|c| c.chain_id).collect(),
    }
}

pub fn map_tools(resp: ToolsResponse) -> ToolsDto {
    ToolsDto {
        bridges: resp.bridges.iter().map(map_bridge_tool).collect(),
        exchanges: resp.exchanges.iter().map(map_exchange_tool).collect(),
    }
}

// ---------------------------------------------------------------------------
// Estimate
// ---------------------------------------------------------------------------

fn map_estimate(raw: &RawEstimate) -> EstimateDto {
    EstimateDto {
        from_amount: raw.from_amount.clone(),
        to_amount: raw.to_amount.clone(),
        to_amount_min: raw.to_amount_min.clone(),
        execution_duration: {
            let d = raw.execution_duration;
            if d.is_nan() || d < 0.0 {
                0u64
            } else if d == f64::INFINITY || d > u64::MAX as f64 {
                u64::MAX
            } else {
                d.round() as u64
            }
        },
        fees: raw
            .fee_costs
            .iter()
            .map(|f| FeeDto {
                amount: f.amount.clone(),
                token: map_token(&f.token),
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Quote (single step returned by /quote)
// ---------------------------------------------------------------------------

pub fn map_quote(raw: QuoteResponse) -> QuoteDto {
    QuoteDto {
        from_chain_id: raw.action.from_chain_id,
        to_chain_id: raw.action.to_chain_id,
        from_token: map_token(&raw.action.from_token),
        to_token: map_token(&raw.action.to_token),
        from_amount: raw.action.from_amount.clone(),
        to_amount: raw.estimate.to_amount.clone(),
        to_amount_min: raw.estimate.to_amount_min.clone(),
        tool: raw.tool.clone(),
        tool_details: ToolDetailsDto {
            key: raw.tool_details.key.clone(),
            name: raw.tool_details.name.clone(),
            logo_uri: raw.tool_details.logo_uri.clone(),
        },
        estimate: map_estimate(&raw.estimate),
    }
}

// ---------------------------------------------------------------------------
// Step
// ---------------------------------------------------------------------------

fn map_step(raw: &RawStep) -> StepDto {
    StepDto {
        step_type: raw.step_type.clone(),
        tool: raw.tool.clone(),
        from_token: map_token(&raw.action.from_token),
        to_token: map_token(&raw.action.to_token),
        from_amount: raw.action.from_amount.clone(),
        to_amount: raw.estimate.to_amount.clone(),
        estimate: map_estimate(&raw.estimate),
    }
}

// ---------------------------------------------------------------------------
// Route
// ---------------------------------------------------------------------------

fn map_route(raw: &RawRoute, chain_types: &std::collections::HashMap<u64, String>) -> RouteDto {
    let mut dto = RouteDto {
        id: raw.id.clone(),
        from_chain_id: raw.from_chain_id,
        to_chain_id: raw.to_chain_id,
        from_token: map_token(&raw.from_token),
        to_token: map_token(&raw.to_token),
        from_amount: raw.from_amount.clone(),
        to_amount: raw.to_amount.clone(),
        to_amount_min: raw.to_amount_min.clone(),
        steps: raw.steps.iter().map(map_step).collect(),
        tags: raw.tags.clone(),
        execution_supported: false,
        execution_family: String::new(),
        execution_reason: None,
    };
    let support = classify_route_with_chain_types(&dto, chain_types);
    dto.execution_supported = support.supported;
    dto.execution_family = support.family.to_string();
    dto.execution_reason = support.reason;
    dto
}

pub fn map_routes(
    resp: &RoutesResponse,
    params_summary: RoutesSummary,
    chain_types: &std::collections::HashMap<u64, String>,
) -> RoutesListDto {
    RoutesListDto {
        routes: resp
            .routes
            .iter()
            .map(|r| map_route(r, chain_types))
            .collect(),
        from_chain_id: params_summary.from_chain_id,
        to_chain_id: params_summary.to_chain_id,
        from_token: params_summary.from_token,
        to_token: params_summary.to_token,
        from_amount: params_summary.from_amount,
    }
}

/// Contextual metadata carried alongside a routes mapping call.
pub struct RoutesSummary {
    pub from_chain_id: u64,
    pub to_chain_id: u64,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

pub fn map_status(raw: StatusResponse) -> StatusDto {
    let sending_tx_hash = raw.sending.as_ref().and_then(|s| s.tx_hash.clone());
    let receiving_tx_hash = raw.receiving.as_ref().and_then(|r| r.tx_hash.clone());
    let from_token = raw
        .sending
        .as_ref()
        .and_then(|s| s.token.as_ref())
        .map(map_token);
    let to_token = raw
        .receiving
        .as_ref()
        .and_then(|r| r.token.as_ref())
        .map(map_token);
    let from_amount = raw.sending.as_ref().and_then(|s| s.amount.clone());
    let to_amount = raw.receiving.as_ref().and_then(|r| r.amount.clone());

    StatusDto {
        status: raw.status,
        substatus: raw.substatus,
        tx_hash: raw.tx_hash,
        sending_tx_hash,
        receiving_tx_hash,
        from_chain_id: raw.from_chain_id,
        to_chain_id: raw.to_chain_id,
        from_token,
        to_token,
        from_amount,
        to_amount,
    }
}
