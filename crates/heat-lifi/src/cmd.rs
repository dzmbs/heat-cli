/// LI.FI command tree.
///
/// Read commands call the LI.FI API and render Heat-owned DTOs.
/// The `bridge` command executes supported EVM-only routes through `heat-evm`.
use clap::{Args, Subcommand};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use heat_core::safety::DryRunPreview;
use heat_evm::EvmChain;

use crate::client::{LifiClient, QuoteParams, RoutesParams, StatusParams};
use crate::dto::{
    BridgeResultDto, ChainsListDto, QuoteDto, RoutesListDto, StatusDto, StepResultDto, TokenDto,
    TokensListDto, ToolsDto,
};
use crate::map::{self, RoutesSummary};

// ---------------------------------------------------------------------------
// Top-level command
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct LifiCmd {
    #[command(subcommand)]
    pub command: LifiSubcommand,
}

#[derive(Subcommand)]
pub enum LifiSubcommand {
    /// List supported chains
    Chains(ChainsArgs),
    /// List supported tokens for a chain
    Tokens(TokensArgs),
    /// List available bridge and DEX tools
    Tools(ToolsArgs),
    /// Get a single-route quote for a token transfer
    Quote(QuoteArgs),
    /// Get multiple route options for a token transfer
    Routes(RoutesArgs),
    /// Check the status of an in-flight transfer
    Status(StatusArgs),
    /// Bridge tokens between EVM chains
    Bridge(BridgeArgs),
}

// ---------------------------------------------------------------------------
// Argument structs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct ChainsArgs {}

#[derive(Args)]
pub struct TokensArgs {
    /// Chain name (ethereum, polygon, arbitrum, base, optimism) or numeric chain ID.
    /// Read commands accept any LI.FI chain; bridge execution requires Heat-supported chains.
    #[arg(long)]
    pub chain: Option<String>,
}

#[derive(Args)]
pub struct ToolsArgs {}

#[derive(Args)]
pub struct QuoteArgs {
    /// Source chain name or numeric ID (reads accept any LI.FI chain)
    #[arg(long)]
    pub from_chain: String,

    /// Destination chain name or numeric ID (reads accept any LI.FI chain)
    #[arg(long)]
    pub to_chain: String,

    /// Source token address or symbol
    #[arg(long)]
    pub from_token: String,

    /// Destination token address or symbol
    #[arg(long)]
    pub to_token: String,

    /// Amount in base units (e.g. wei for ETH)
    #[arg(long)]
    pub amount: String,

    /// Sender address (required for accurate quotes)
    #[arg(long)]
    pub from_address: String,
}

#[derive(Args)]
pub struct RoutesArgs {
    /// Source chain name or numeric ID (reads accept any LI.FI chain)
    #[arg(long)]
    pub from_chain: String,

    /// Destination chain name or numeric ID (reads accept any LI.FI chain)
    #[arg(long)]
    pub to_chain: String,

    /// Source token address or symbol
    #[arg(long)]
    pub from_token: String,

    /// Destination token address or symbol
    #[arg(long)]
    pub to_token: String,

    /// Amount in base units (e.g. wei for ETH)
    #[arg(long)]
    pub amount: String,

    /// Sender address (optional)
    #[arg(long)]
    pub from_address: Option<String>,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Transaction hash of the sending transaction
    #[arg(long)]
    pub tx_hash: String,

    /// Bridge name (required for cross-chain transfers)
    #[arg(long)]
    pub bridge: Option<String>,

    /// Source chain name or numeric ID (reads accept any LI.FI chain)
    #[arg(long)]
    pub from_chain: String,

    /// Destination chain name or numeric ID (reads accept any LI.FI chain)
    #[arg(long)]
    pub to_chain: String,
}

#[derive(Args)]
pub struct BridgeArgs {
    /// Amount in human-readable units (e.g. 100.5 for 100.5 USDC)
    pub amount: String,

    /// Source token symbol or address (e.g. USDC, ETH, 0x…)
    pub token: String,

    /// Source chain (ethereum, polygon, arbitrum, base, optimism)
    #[arg(long)]
    pub from: String,

    /// Destination chain
    #[arg(long)]
    pub to: String,

    /// Destination token symbol or address (defaults to same token)
    #[arg(long)]
    pub to_token: Option<String>,

    /// Select a specific route by index (0-based) from the routes list
    #[arg(long)]
    pub route_index: Option<usize>,

    /// Source chain RPC URL override
    #[arg(long)]
    pub rpc: Option<String>,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub async fn run(cmd: LifiCmd, ctx: &Ctx) -> Result<(), HeatError> {
    match cmd.command {
        LifiSubcommand::Chains(args) => chains(args, ctx).await,
        LifiSubcommand::Tokens(args) => tokens(args, ctx).await,
        LifiSubcommand::Tools(args) => tools(args, ctx).await,
        LifiSubcommand::Quote(args) => quote(args, ctx).await,
        LifiSubcommand::Routes(args) => routes(args, ctx).await,
        LifiSubcommand::Status(args) => status(args, ctx).await,
        LifiSubcommand::Bridge(args) => bridge(args, ctx).await,
    }
}

// ---------------------------------------------------------------------------
// Read commands
// ---------------------------------------------------------------------------

async fn chains(_args: ChainsArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let client = LifiClient::new()?;
    let raw = client.chains().await?;
    let dto = map::map_chains(raw);

    ctx.output
        .write_data(&dto, Some(&pretty_chains))
        .map_err(io_err)
}

async fn tokens(args: TokensArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let chain_id = match &args.chain {
        Some(s) => Some(resolve_chain_id(s)?),
        None => None,
    };
    let client = LifiClient::new()?;
    let raw = client.tokens(chain_id).await?;
    let dto = map::map_tokens(raw, chain_id);

    ctx.output
        .write_data(&dto, Some(&pretty_tokens))
        .map_err(io_err)
}

async fn tools(_args: ToolsArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let client = LifiClient::new()?;
    let raw = client.tools().await?;
    let dto = map::map_tools(raw);

    ctx.output
        .write_data(&dto, Some(&pretty_tools))
        .map_err(io_err)
}

async fn quote(args: QuoteArgs, ctx: &Ctx) -> Result<(), HeatError> {
    validate_amount(&args.amount)?;
    let from_chain = resolve_chain_id(&args.from_chain)?.to_string();
    let to_chain = resolve_chain_id(&args.to_chain)?.to_string();

    let client = LifiClient::new()?;
    let params = QuoteParams {
        from_chain,
        to_chain,
        from_token: args.from_token,
        to_token: args.to_token,
        from_amount: args.amount,
        from_address: Some(args.from_address),
    };
    let raw = client.quote(&params).await?;
    let dto = map::map_quote(raw);

    ctx.output
        .write_data(&dto, Some(&pretty_quote))
        .map_err(io_err)
}

async fn routes(args: RoutesArgs, ctx: &Ctx) -> Result<(), HeatError> {
    validate_amount(&args.amount)?;
    let from_chain_id = resolve_chain_id(&args.from_chain)?;
    let to_chain_id = resolve_chain_id(&args.to_chain)?;

    let client = LifiClient::new()?;

    let chains_resp = client.chains().await?;
    let chain_types: std::collections::HashMap<u64, String> = chains_resp
        .chains
        .iter()
        .map(|c| (c.id, c.chain_type.clone()))
        .collect();

    let params = RoutesParams {
        from_chain_id,
        to_chain_id,
        from_token_address: args.from_token.clone(),
        to_token_address: args.to_token.clone(),
        from_amount: args.amount.clone(),
        from_address: args.from_address,
    };
    let summary = RoutesSummary {
        from_chain_id,
        to_chain_id,
        from_token: args.from_token,
        to_token: args.to_token,
        from_amount: args.amount,
    };

    let raw = client.routes(&params).await?;
    let dto = map::map_routes(raw, summary, &chain_types);

    ctx.output
        .write_data(&dto, Some(&pretty_routes))
        .map_err(io_err)
}

async fn status(args: StatusArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let from_chain = resolve_chain_id(&args.from_chain)?.to_string();
    let to_chain = resolve_chain_id(&args.to_chain)?.to_string();

    let client = LifiClient::new()?;
    let params = StatusParams {
        tx_hash: args.tx_hash,
        bridge: args.bridge,
        from_chain,
        to_chain,
    };
    let raw = client.status(&params).await?;
    let dto = map::map_status(raw);

    if ctx.output.format == OutputFormat::Quiet {
        return ctx.output.write_scalar(&dto.status).map_err(io_err);
    }

    ctx.output
        .write_data(&dto, Some(&pretty_status))
        .map_err(io_err)
}

// ---------------------------------------------------------------------------
// Bridge command
// ---------------------------------------------------------------------------

async fn bridge(args: BridgeArgs, ctx: &Ctx) -> Result<(), HeatError> {
    // 1. Resolve chains.
    let from_chain = EvmChain::from_name(&args.from)?;
    let to_chain = EvmChain::from_name(&args.to)?;
    let from_chain_id = from_chain.chain_id();
    let to_chain_id = to_chain.chain_id();

    // 2. Resolve sender address from Heat account.
    let sender = heat_evm::resolve_eoa_address(ctx)?;
    let sender_hex = format!("{sender:#x}");

    // 3. Resolve source token via LI.FI token list.
    let client = LifiClient::new()?;
    let (from_token_addr, from_token_symbol, from_token_decimals) =
        resolve_token(&client, from_chain_id, &args.token).await?;

    // 4. Resolve destination token (same symbol by default).
    let to_token_input = args.to_token.as_deref().unwrap_or(&args.token);
    let (to_token_addr, _to_token_symbol, _to_token_decimals) =
        resolve_token(&client, to_chain_id, to_token_input).await?;

    // 5. Parse amount.
    let amount_base = heat_evm::amount::parse_units(&args.amount, from_token_decimals)?;
    if amount_base.is_zero() {
        return Err(HeatError::validation(
            "zero_amount",
            "Bridge amount must be greater than zero",
        ));
    }
    let amount_str = amount_base.to_string();
    let amount_display = format!("{} {}", args.amount, from_token_symbol);

    // 6. Request routes.
    let chains_resp = client.chains().await?;
    let chain_types: std::collections::HashMap<u64, String> = chains_resp
        .chains
        .iter()
        .map(|c| (c.id, c.chain_type.clone()))
        .collect();

    let routes_params = RoutesParams {
        from_chain_id,
        to_chain_id,
        from_token_address: from_token_addr.clone(),
        to_token_address: to_token_addr.clone(),
        from_amount: amount_str.clone(),
        from_address: Some(sender_hex.clone()),
    };
    let routes_resp = client.routes(&routes_params).await?;

    if routes_resp.routes.is_empty() {
        return Err(HeatError::protocol(
            "no_routes",
            format!(
                "No routes found for {} → {} ({} → {})",
                from_chain.canonical_name(),
                to_chain.canonical_name(),
                from_token_symbol,
                to_token_input,
            ),
        )
        .with_hint("Try a different token pair or amount"));
    }

    // 7. Map routes and filter to executable ones.
    let summary = RoutesSummary {
        from_chain_id,
        to_chain_id,
        from_token: from_token_addr.clone(),
        to_token: to_token_addr.clone(),
        from_amount: amount_str.clone(),
    };
    let all_routes = map::map_routes(routes_resp, summary, &chain_types);
    let executable: Vec<_> = all_routes
        .routes
        .iter()
        .enumerate()
        .filter(|(_, r)| r.execution_supported)
        .collect();

    if executable.is_empty() {
        return Err(HeatError::protocol(
            "no_executable_routes",
            "No executable routes found — all returned routes require unsupported chain families",
        )
        .with_hint("Heat currently supports EVM-only routes. Run 'heat lifi routes' to inspect available routes."));
    }

    // 8. Select route.
    let (route_idx, selected_route) = if let Some(idx) = args.route_index {
        executable
            .iter()
            .find(|(i, _)| *i == idx)
            .ok_or_else(|| {
                HeatError::validation(
                    "invalid_route_index",
                    format!(
                        "Route index {idx} is not executable. Executable indices: {}",
                        executable
                            .iter()
                            .map(|(i, _)| i.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            })
            .map(|(i, r)| (*i, *r))?
    } else {
        let (i, r) = executable[0];
        (i, r)
    };

    let tools_used: Vec<String> = selected_route
        .steps
        .iter()
        .map(|s| s.tool.clone())
        .collect();

    // 9. Dry-run preview.
    if ctx.dry_run {
        DryRunPreview::new("lifi", "bridge")
            .param("from", from_chain.canonical_name())
            .param("to", to_chain.canonical_name())
            .param(
                "token",
                &format!("{} ({})", from_token_symbol, from_token_addr),
            )
            .param("amount", &amount_display)
            .param(
                "route",
                &format!("{} (index {})", selected_route.id, route_idx),
            )
            .param("steps", &selected_route.steps.len().to_string())
            .param("tools", &tools_used.join(", "))
            .param("estimated output", &selected_route.to_amount)
            .param("minimum output", &selected_route.to_amount_min)
            .display();
        return Ok(());
    }

    // 10. Confirmation.
    ctx.confirm_dangerous(&format!(
        "bridge {} from {} to {}",
        amount_display,
        from_chain.canonical_name(),
        to_chain.canonical_name(),
    ))?;

    // 11. Build wallet provider for source chain.
    let rpc_url =
        heat_evm::rpc::resolve_rpc_url(ctx, from_chain, args.rpc.as_deref(), Some("lifi"))?;
    let wallet_prov = heat_evm::wallet_provider(ctx, from_chain, &rpc_url).await?;

    // 12. Re-fetch raw routes to get raw step data for stepTransaction API.
    let raw_routes = client
        .routes(&RoutesParams {
            from_chain_id,
            to_chain_id,
            from_token_address: from_token_addr.clone(),
            to_token_address: to_token_addr.clone(),
            from_amount: amount_str.clone(),
            from_address: Some(sender_hex.clone()),
        })
        .await?;

    // Find the same route by ID.
    let raw_route = raw_routes
        .routes
        .into_iter()
        .find(|r| r.id == selected_route.id)
        .ok_or_else(|| {
            HeatError::protocol(
                "route_expired",
                "Selected route is no longer available (routes may expire quickly)",
            )
            .with_hint("Try again — routes are time-sensitive")
        })?;

    // 13. Execute each step.
    let mut step_results = Vec::new();

    for (step_idx, raw_step) in raw_route.steps.iter().enumerate() {
        let step_label = format!(
            "step {}/{} ({})",
            step_idx + 1,
            raw_route.steps.len(),
            raw_step.tool,
        );
        ctx.output.diagnostic(&format!("Executing {step_label}..."));

        // Build step JSON for the stepTransaction API.
        // Inject action.fromAddress — required by LI.FI for sender-sensitive flows.
        let mut step_json = serde_json::to_value(raw_step).map_err(|e| {
            HeatError::internal("step_serialize", format!("Failed to serialize step: {e}"))
        })?;
        if let Some(action) = step_json.get_mut("action") {
            action["fromAddress"] = serde_json::Value::String(sender_hex.clone());
        }

        // Request transaction data from LI.FI.
        let step_tx = client.step_transaction(&step_json).await?;

        // Verify chain ID matches source chain.
        if step_tx.transaction_request.chain_id != from_chain_id {
            return Err(HeatError::protocol(
                "cross_chain_step",
                format!(
                    "Step {} requires execution on chain {} but source chain is {} ({})",
                    step_idx + 1,
                    step_tx.transaction_request.chain_id,
                    from_chain_id,
                    from_chain.canonical_name(),
                ),
            )
            .with_hint("Multi-chain step execution is not yet supported. Try a simpler route."));
        }

        // Handle ERC-20 approval if needed.
        let mut approval_tx_hash = None;
        if let Some(approval_addr) = &step_tx.estimate.approval_address {
            let from_token_address: alloy::primitives::Address =
                step_tx.action.from_token.address.parse().map_err(|_| {
                    HeatError::protocol(
                        "invalid_token_address",
                        format!(
                            "Invalid token address in step: {}",
                            step_tx.action.from_token.address
                        ),
                    )
                })?;

            // Skip approval for native token (zero address or LI.FI native placeholder).
            let is_native = from_token_address.is_zero()
                || step_tx.action.from_token.address.to_lowercase()
                    == "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

            if !is_native {
                let spender: alloy::primitives::Address = approval_addr.parse().map_err(|_| {
                    HeatError::protocol(
                        "invalid_approval_address",
                        format!("Invalid approval address: {approval_addr}"),
                    )
                })?;

                let amount_to_approve: alloy::primitives::U256 =
                    step_tx.action.from_amount.parse().map_err(|_| {
                        HeatError::protocol(
                            "invalid_step_amount",
                            format!("Invalid amount in step: {}", step_tx.action.from_amount),
                        )
                    })?;

                let current_allowance =
                    heat_evm::erc20::allowance(&wallet_prov, from_token_address, sender, spender)
                        .await?;

                if current_allowance < amount_to_approve {
                    ctx.output.diagnostic(&format!(
                        "Approving {} for {}...",
                        step_tx.action.from_token.symbol, raw_step.tool,
                    ));
                    let tx_hash = heat_evm::erc20::approve(
                        &wallet_prov,
                        from_token_address,
                        spender,
                        amount_to_approve,
                    )
                    .await?;
                    ctx.output.diagnostic(&format!("Approval tx: {tx_hash:#x}"));
                    approval_tx_hash = Some(format!("{tx_hash:#x}"));
                }
            }
        }

        // Send the step transaction.
        let to_addr: alloy::primitives::Address =
            step_tx.transaction_request.to.parse().map_err(|_| {
                HeatError::protocol(
                    "invalid_tx_to",
                    format!("Invalid 'to' address: {}", step_tx.transaction_request.to),
                )
            })?;

        let calldata = alloy::primitives::Bytes::from(
            alloy::primitives::hex::decode(&step_tx.transaction_request.data).map_err(|_| {
                HeatError::protocol(
                    "invalid_tx_data",
                    "Invalid hex calldata in step transaction",
                )
            })?,
        );

        let value: alloy::primitives::U256 =
            parse_value_flexible(&step_tx.transaction_request.value)?;

        let mut tx_request = alloy::rpc::types::TransactionRequest::default()
            .to(to_addr)
            .input(alloy::rpc::types::TransactionInput::new(calldata))
            .value(value);

        if let Some(gas_limit) = &step_tx.transaction_request.gas_limit {
            let gl = parse_value_flexible(gas_limit)?;
            let gl_u64: u64 = gl.try_into().map_err(|_| {
                HeatError::protocol(
                    "invalid_gas_limit",
                    format!("Gas limit too large: {gas_limit}"),
                )
            })?;
            tx_request = tx_request.gas_limit(gl_u64);
        }

        let pending = alloy::providers::Provider::send_transaction(&wallet_prov, tx_request)
            .await
            .map_err(|e| {
                HeatError::network(
                    "lifi_step_send",
                    format!("Failed to send step {} transaction: {e}", step_idx + 1),
                )
            })?;

        let receipt = pending.get_receipt().await.map_err(|e| {
            HeatError::network(
                "lifi_step_receipt",
                format!("Failed to get step {} receipt: {e}", step_idx + 1),
            )
        })?;

        if !receipt.status() {
            return Err(HeatError::protocol(
                "lifi_step_reverted",
                format!(
                    "Step {} transaction reverted (tx: {:#x})",
                    step_idx + 1,
                    receipt.transaction_hash,
                ),
            ));
        }

        let tx_hash = format!("{:#x}", receipt.transaction_hash);
        ctx.output
            .diagnostic(&format!("{step_label} complete: {tx_hash}"));

        step_results.push(StepResultDto {
            step_type: raw_step.step_type.clone(),
            tool: raw_step.tool.clone(),
            tx_hash,
            approval_tx_hash,
        });
    }

    // 14. Build result DTO.
    let dto = BridgeResultDto {
        from_chain: from_chain.canonical_name().to_owned(),
        to_chain: to_chain.canonical_name().to_owned(),
        from_token: selected_route.from_token.clone(),
        to_token: selected_route.to_token.clone(),
        from_amount: amount_str,
        from_amount_display: amount_display.clone(),
        to_amount_estimate: selected_route.to_amount.clone(),
        to_amount_min: selected_route.to_amount_min.clone(),
        route_id: selected_route.id.clone(),
        route_tags: selected_route.tags.clone(),
        tools_used,
        account: sender_hex,
        step_results,
    };

    ctx.output
        .write_data(&dto, Some(&pretty_bridge))
        .map_err(io_err)
}

// ---------------------------------------------------------------------------
// Chain / token resolution helpers
// ---------------------------------------------------------------------------

/// Resolve a chain input to a numeric chain ID.
/// Accepts Heat chain names (ethereum, polygon, etc.) and numeric IDs.
pub(crate) fn resolve_chain_id(input: &str) -> Result<u64, HeatError> {
    // Try as Heat chain name first.
    if let Ok(chain) = EvmChain::from_name(input) {
        return Ok(chain.chain_id());
    }
    // Fall back to numeric parsing.
    input.parse::<u64>().map_err(|_| {
        HeatError::validation(
            "invalid_chain",
            format!("'{input}' is not a valid chain name or numeric chain ID"),
        )
        .with_hint("Use a chain name (ethereum, polygon, arbitrum, base, optimism) or numeric ID")
    })
}

/// Resolve a token symbol or address on a specific chain via the LI.FI token list.
/// Returns (address, symbol, decimals).
async fn resolve_token(
    client: &LifiClient,
    chain_id: u64,
    input: &str,
) -> Result<(String, String, u8), HeatError> {
    let trimmed = input.trim();

    // If it looks like an address, return it directly with metadata from LI.FI.
    if trimmed.starts_with("0x") && trimmed.len() == 42 {
        let tokens_resp = client.tokens(Some(chain_id)).await?;
        let all_tokens: Vec<_> = tokens_resp.tokens.values().flatten().collect();
        if let Some(t) = all_tokens
            .iter()
            .find(|t| t.address.eq_ignore_ascii_case(trimmed))
        {
            return Ok((t.address.clone(), t.symbol.clone(), t.decimals));
        }
        // Address not in LI.FI token list — use it anyway but fetch decimals from chain
        // would require an RPC call. For now, return an error.
        return Err(HeatError::validation(
            "unknown_token_address",
            format!("Token address {trimmed} not found in LI.FI's token list for chain {chain_id}"),
        )
        .with_hint("Use a known token symbol (e.g., USDC, ETH) or verify the address"));
    }

    // Symbol lookup.
    let tokens_resp = client.tokens(Some(chain_id)).await?;
    let all_tokens: Vec<_> = tokens_resp.tokens.values().flatten().collect();
    let input_upper = trimmed.to_uppercase();
    let matched: Vec<_> = all_tokens
        .iter()
        .filter(|t| t.symbol.to_uppercase() == input_upper)
        .collect();

    match matched.len() {
        0 => Err(HeatError::validation(
            "unknown_token",
            format!("No token with symbol '{trimmed}' found on chain {chain_id}"),
        )
        .with_hint("Use the exact symbol (e.g., USDC, WETH, DAI) or pass the token address")),
        1 => {
            let t = matched[0];
            Ok((t.address.clone(), t.symbol.clone(), t.decimals))
        }
        _ => Err(HeatError::validation(
            "ambiguous_token",
            format!(
                "Multiple tokens match '{}' on chain {}: {}",
                trimmed,
                chain_id,
                matched
                    .iter()
                    .map(|t| format!("{} ({})", t.symbol, &t.address[..10]))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
        .with_hint("Pass the exact token address to disambiguate")),
    }
}

/// Parse a value string that may be decimal or hex (0x-prefixed).
pub(crate) fn parse_value_flexible(s: &str) -> Result<alloy::primitives::U256, HeatError> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed == "0" || trimmed == "0x0" || trimmed == "0x" {
        return Ok(alloy::primitives::U256::ZERO);
    }
    if let Some(hex) = trimmed.strip_prefix("0x") {
        alloy::primitives::U256::from_str_radix(hex, 16).map_err(|_| {
            HeatError::protocol("invalid_tx_value", format!("Invalid hex value: {trimmed}"))
        })
    } else {
        trimmed.parse::<alloy::primitives::U256>().map_err(|_| {
            HeatError::protocol(
                "invalid_tx_value",
                format!("Invalid transaction value: {trimmed}"),
            )
        })
    }
}

// ---------------------------------------------------------------------------
// Pretty formatters
// ---------------------------------------------------------------------------

fn pretty_chains(dto: &ChainsListDto) -> String {
    let mut out = format!("{:<8} {:<30} {:<8} {}\n", "ID", "NAME", "TYPE", "NATIVE");
    out.push_str(&"-".repeat(65));
    out.push('\n');
    for chain in &dto.chains {
        out.push_str(&format!(
            "{:<8} {:<30} {:<8} {}\n",
            chain.id,
            truncate(&chain.name, 29),
            chain.chain_type,
            chain.native_token.symbol,
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_tokens(dto: &TokensListDto) -> String {
    let header = if let Some(id) = dto.chain_id {
        format!("Tokens on chain {id} ({} total)\n", dto.tokens.len())
    } else {
        format!("All tokens ({} total)\n", dto.tokens.len())
    };
    let mut out = header;
    out.push_str(&format!(
        "{:<10} {:<42} {:<5} {}\n",
        "SYMBOL", "ADDRESS", "DEC", "NAME"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');
    for token in &dto.tokens {
        out.push_str(&format!(
            "{:<10} {:<42} {:<5} {}\n",
            truncate(&token.symbol, 9),
            token.address,
            token.decimals,
            truncate(&token.name, 25),
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_tools(dto: &ToolsDto) -> String {
    let mut out = format!("Bridges ({}):\n", dto.bridges.len());
    for b in &dto.bridges {
        out.push_str(&format!(
            "  {:<20} chains: {}\n",
            b.name,
            b.supported_chains.len()
        ));
    }
    out.push('\n');
    out.push_str(&format!("Exchanges ({}):\n", dto.exchanges.len()));
    for e in &dto.exchanges {
        out.push_str(&format!(
            "  {:<20} chains: {}\n",
            e.name,
            e.supported_chains.len()
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_token_inline(t: &TokenDto) -> String {
    format!("{} (chain {})", t.symbol, t.chain_id)
}

fn pretty_quote(dto: &QuoteDto) -> String {
    format!(
        "Quote via {tool}\n\
         From:    {from_token}\n\
         To:      {to_token}\n\
         Input:   {from_amount}\n\
         Output:  {to_amount}\n\
         Min out: {to_amount_min}\n\
         ETA:     {eta}s\n\
         Fees:    {fee_count} fee(s)",
        tool = dto.tool_details.name,
        from_token = pretty_token_inline(&dto.from_token),
        to_token = pretty_token_inline(&dto.to_token),
        from_amount = dto.from_amount,
        to_amount = dto.estimate.to_amount,
        to_amount_min = dto.estimate.to_amount_min,
        eta = dto.estimate.execution_duration,
        fee_count = dto.estimate.fees.len(),
    )
}

fn pretty_routes(dto: &RoutesListDto) -> String {
    let mut out = format!(
        "{} route(s) from chain {} → chain {}\n\n",
        dto.routes.len(),
        dto.from_chain_id,
        dto.to_chain_id
    );
    for (i, route) in dto.routes.iter().enumerate() {
        let tags = if route.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", route.tags.join(", "))
        };
        let execution_line = if route.execution_supported {
            format!("supported ({})", route.execution_family)
        } else {
            let reason = route.execution_reason.as_deref().unwrap_or("not supported");
            format!("not supported — {reason}")
        };
        out.push_str(&format!(
            "{}. {}{}\n   {} → {}\n   In: {}  Out: {}  Steps: {}\n   Execution: {}\n\n",
            i + 1,
            route.id,
            tags,
            pretty_token_inline(&route.from_token),
            pretty_token_inline(&route.to_token),
            route.from_amount,
            route.to_amount,
            route.steps.len(),
            execution_line,
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_status(dto: &StatusDto) -> String {
    let mut out = format!("Status: {}", dto.status);
    if !dto.substatus.is_empty() {
        out.push_str(&format!(" / {}", dto.substatus));
    }
    out.push('\n');
    if let Some(tx) = &dto.tx_hash {
        out.push_str(&format!("Tx hash:  {tx}\n"));
    }
    if let Some(tx) = &dto.sending_tx_hash {
        out.push_str(&format!("Sending:  {tx}\n"));
    }
    if let Some(tx) = &dto.receiving_tx_hash {
        out.push_str(&format!("Received: {tx}\n"));
    }
    if let (Some(from), Some(to)) = (&dto.from_amount, &dto.to_amount) {
        let from_sym = dto
            .from_token
            .as_ref()
            .map(|t| t.symbol.as_str())
            .unwrap_or("?");
        let to_sym = dto
            .to_token
            .as_ref()
            .map(|t| t.symbol.as_str())
            .unwrap_or("?");
        out.push_str(&format!("Amount:   {from} {from_sym} → {to} {to_sym}\n"));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_bridge(dto: &BridgeResultDto) -> String {
    let mut out = format!(
        "Bridged {} from {} to {}\n\
         Route:   {} [{}]\n\
         Tools:   {}\n\
         Account: {}\n\
         Est. output: {}\n\
         Min. output: {}",
        dto.from_amount_display,
        dto.from_chain,
        dto.to_chain,
        dto.route_id,
        if dto.route_tags.is_empty() {
            "no tags".to_owned()
        } else {
            dto.route_tags.join(", ")
        },
        dto.tools_used.join(" → "),
        dto.account,
        dto.to_amount_estimate,
        dto.to_amount_min,
    );
    for (i, step) in dto.step_results.iter().enumerate() {
        out.push_str(&format!(
            "\nStep {}: {} via {} — tx: {}",
            i + 1,
            step.step_type,
            step.tool,
            step.tx_hash,
        ));
        if let Some(approval) = &step.approval_tx_hash {
            out.push_str(&format!(" (approval: {approval})"));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate that an amount string is a non-empty, non-negative integer (base-unit amount).
pub(crate) fn validate_amount(input: &str) -> Result<(), HeatError> {
    if input.is_empty() {
        return Err(HeatError::validation(
            "empty_amount",
            "Amount must not be empty",
        ));
    }
    if input.starts_with('-') {
        return Err(HeatError::validation(
            "negative_amount",
            "Amount must not be negative",
        ));
    }
    if !input.chars().all(|c| c.is_ascii_digit()) {
        return Err(HeatError::validation(
            "invalid_amount",
            format!("Amount must be a non-negative integer in base units, got '{input}'"),
        )
        .with_hint(
            "Pass the amount in the smallest token unit (e.g. wei for ETH, lamports for SOL)",
        ));
    }
    if input.len() > 1 && input.starts_with('0') {
        return Err(HeatError::validation(
            "invalid_amount",
            "Amount must not have leading zeros",
        ));
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
