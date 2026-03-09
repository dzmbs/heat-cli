/// Aave V3 command tree.
///
/// Commands follow Heat conventions:
/// - Read-only: markets, positions, health
/// - Write: supply, withdraw (with --dry-run, --yes, confirmation)
///
/// All commands resolve Pool and DataProvider addresses at runtime
/// from the on-chain PoolAddressesProvider.
use clap::{Args, Subcommand};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use heat_core::safety::DryRunPreview;
use heat_evm::EvmChain;

use crate::addresses;
use crate::dto::{HealthDto, MarketsListDto, PositionsListDto, SupplyResultDto, WithdrawResultDto};
use crate::resolver;

// ---------------------------------------------------------------------------
// Top-level command
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct AaveCmd {
    #[command(subcommand)]
    pub command: AaveSubcommand,
}

#[derive(Subcommand)]
pub enum AaveSubcommand {
    /// List supported reserve markets
    Markets(MarketsArgs),
    /// Show account's per-reserve positions
    Positions(PositionsArgs),
    /// Show account health summary
    Health(HealthArgs),
    /// Supply assets into the pool
    Supply(SupplyArgs),
    /// Withdraw assets from the pool
    Withdraw(WithdrawArgs),
}

// ---------------------------------------------------------------------------
// Argument structs
// ---------------------------------------------------------------------------

#[derive(Args)]
pub struct MarketsArgs {
    /// Chain name (ethereum, arbitrum, base)
    #[arg(long)]
    pub chain: Option<String>,

    /// RPC URL override
    #[arg(long)]
    pub rpc: Option<String>,
}

#[derive(Args)]
pub struct PositionsArgs {
    /// Chain name (ethereum, arbitrum, base)
    #[arg(long)]
    pub chain: Option<String>,

    /// RPC URL override
    #[arg(long)]
    pub rpc: Option<String>,
}

#[derive(Args)]
pub struct HealthArgs {
    /// Chain name (ethereum, arbitrum, base)
    #[arg(long)]
    pub chain: Option<String>,

    /// RPC URL override
    #[arg(long)]
    pub rpc: Option<String>,
}

#[derive(Args)]
pub struct SupplyArgs {
    /// Asset symbol or address (e.g. USDC, WETH, 0x…)
    pub asset: String,

    /// Amount in human-readable units (e.g. 100.5 for 100.5 USDC)
    pub amount: String,

    /// Chain name (ethereum, arbitrum, base)
    #[arg(long)]
    pub chain: Option<String>,

    /// RPC URL override
    #[arg(long)]
    pub rpc: Option<String>,
}

#[derive(Args)]
pub struct WithdrawArgs {
    /// Asset symbol or address (e.g. USDC, WETH, 0x…)
    pub asset: String,

    /// Amount in human-readable units (e.g. 100.5 for 100.5 USDC)
    pub amount: String,

    /// Chain name (ethereum, arbitrum, base)
    #[arg(long)]
    pub chain: Option<String>,

    /// RPC URL override
    #[arg(long)]
    pub rpc: Option<String>,
}

// ---------------------------------------------------------------------------
// Chain resolution
// ---------------------------------------------------------------------------

/// Resolve chain from --chain flag, falling back to ctx.network (global --network).
pub(crate) fn resolve_chain(explicit: Option<&str>, ctx: &Ctx) -> Result<EvmChain, HeatError> {
    let name = explicit.or(ctx.network.as_deref()).ok_or_else(|| {
        HeatError::validation("no_chain", "No chain specified")
            .with_hint("Use --chain <NAME> or set a default network with 'heat config' / --network")
    })?;
    EvmChain::from_name(name)
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub async fn run(cmd: AaveCmd, ctx: &Ctx) -> Result<(), HeatError> {
    match cmd.command {
        AaveSubcommand::Markets(args) => markets(args, ctx).await,
        AaveSubcommand::Positions(args) => positions(args, ctx).await,
        AaveSubcommand::Health(args) => health(args, ctx).await,
        AaveSubcommand::Supply(args) => supply(args, ctx).await,
        AaveSubcommand::Withdraw(args) => withdraw(args, ctx).await,
    }
}

// ---------------------------------------------------------------------------
// Read commands
// ---------------------------------------------------------------------------

async fn markets(args: MarketsArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let chain = resolve_chain(args.chain.as_deref(), ctx)?;
    let market = addresses::market_for_chain(chain)?;
    let rpc_url = heat_evm::rpc::resolve_rpc_url(ctx, chain, args.rpc.as_deref(), Some("aave"))?;
    let provider = heat_evm::signer::read_provider(chain, &rpc_url).await?;

    let resolved = resolver::resolve(&provider, market).await?;
    let dto = crate::read::fetch_markets(provider, chain, &resolved).await?;
    ctx.output
        .write_data(&dto, Some(&pretty_markets))
        .map_err(io_err)
}

async fn positions(args: PositionsArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let chain = resolve_chain(args.chain.as_deref(), ctx)?;
    let market = addresses::market_for_chain(chain)?;
    let rpc_url = heat_evm::rpc::resolve_rpc_url(ctx, chain, args.rpc.as_deref(), Some("aave"))?;
    let provider = heat_evm::signer::read_provider(chain, &rpc_url).await?;
    let user = heat_evm::signer::resolve_eoa_address(ctx)?;

    let resolved = resolver::resolve(&provider, market).await?;
    let dto = crate::read::fetch_positions(provider, chain, &resolved, user).await?;
    ctx.output
        .write_data(&dto, Some(&pretty_positions))
        .map_err(io_err)
}

async fn health(args: HealthArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let chain = resolve_chain(args.chain.as_deref(), ctx)?;
    let market = addresses::market_for_chain(chain)?;
    let rpc_url = heat_evm::rpc::resolve_rpc_url(ctx, chain, args.rpc.as_deref(), Some("aave"))?;
    let provider = heat_evm::signer::read_provider(chain, &rpc_url).await?;
    let user = heat_evm::signer::resolve_eoa_address(ctx)?;

    let resolved = resolver::resolve(&provider, market).await?;
    let dto = crate::read::fetch_health(provider, chain, &resolved, user).await?;

    if ctx.output.format == OutputFormat::Quiet {
        return ctx
            .output
            .write_scalar(&dto.health_factor_display)
            .map_err(io_err);
    }

    ctx.output
        .write_data(&dto, Some(&pretty_health))
        .map_err(io_err)
}

// ---------------------------------------------------------------------------
// Write commands
// ---------------------------------------------------------------------------

async fn supply(args: SupplyArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let chain = resolve_chain(args.chain.as_deref(), ctx)?;
    let market = addresses::market_for_chain(chain)?;
    let rpc_url = heat_evm::rpc::resolve_rpc_url(ctx, chain, args.rpc.as_deref(), Some("aave"))?;

    // Resolve addresses from provider, then resolve asset.
    let read_prov = heat_evm::signer::read_provider(chain, &rpc_url).await?;
    let resolved = resolver::resolve(&read_prov, market).await?;
    let (asset_addr, symbol, decimals) =
        crate::read::resolve_asset(read_prov, &resolved, &args.asset).await?;

    // Parse amount.
    let amount = heat_evm::amount::parse_units(&args.amount, decimals)?;
    if amount.is_zero() {
        return Err(HeatError::validation(
            "zero_amount",
            "Supply amount must be greater than zero",
        ));
    }
    let amount_display = format!("{} {}", args.amount, symbol);

    // Dry-run check.
    if ctx.dry_run {
        DryRunPreview::new("aave", "supply")
            .param("chain", chain.canonical_name())
            .param("asset", &format!("{symbol} ({:#x})", asset_addr))
            .param("amount", &amount_display)
            .display();
        return Ok(());
    }

    // Confirmation.
    ctx.confirm_dangerous(&format!("supply {amount_display} on {chain}"))?;

    // Build wallet provider for signing.
    let wallet_prov = heat_evm::signer::wallet_provider(ctx, chain, &rpc_url).await?;
    let user = heat_evm::signer::resolve_eoa_address(ctx)?;

    // Check and handle ERC-20 approval.
    let current_allowance =
        heat_evm::erc20::allowance(&wallet_prov, asset_addr, user, resolved.pool).await?;

    let mut approval_tx = None;
    if current_allowance < amount {
        ctx.output
            .diagnostic(&format!("Approving {symbol} for Aave Pool..."));
        let tx_hash =
            heat_evm::erc20::approve(&wallet_prov, asset_addr, resolved.pool, amount).await?;
        ctx.output
            .diagnostic(&format!("Approval tx: {:#x}", tx_hash));
        approval_tx = Some(format!("{:#x}", tx_hash));
    }

    // Execute supply.
    ctx.output
        .diagnostic(&format!("Supplying {amount_display}..."));
    let tx_hash =
        crate::write::supply(&wallet_prov, resolved.pool, asset_addr, amount, user).await?;

    let dto = SupplyResultDto {
        chain: chain.canonical_name().to_owned(),
        account: format!("{:#x}", user),
        asset_symbol: symbol,
        asset_address: format!("{:#x}", asset_addr),
        amount: amount.to_string(),
        amount_display,
        tx_hash: format!("{:#x}", tx_hash),
        approval_tx_hash: approval_tx,
    };

    ctx.output
        .write_data(&dto, Some(&pretty_supply))
        .map_err(io_err)
}

async fn withdraw(args: WithdrawArgs, ctx: &Ctx) -> Result<(), HeatError> {
    let chain = resolve_chain(args.chain.as_deref(), ctx)?;
    let market = addresses::market_for_chain(chain)?;
    let rpc_url = heat_evm::rpc::resolve_rpc_url(ctx, chain, args.rpc.as_deref(), Some("aave"))?;

    // Resolve addresses from provider, then resolve asset.
    let read_prov = heat_evm::signer::read_provider(chain, &rpc_url).await?;
    let resolved = resolver::resolve(&read_prov, market).await?;
    let (asset_addr, symbol, decimals) =
        crate::read::resolve_asset(read_prov, &resolved, &args.asset).await?;

    // Parse amount.
    let amount = heat_evm::amount::parse_units(&args.amount, decimals)?;
    if amount.is_zero() {
        return Err(HeatError::validation(
            "zero_amount",
            "Withdraw amount must be greater than zero",
        ));
    }
    let amount_display = format!("{} {}", args.amount, symbol);

    // Dry-run check.
    if ctx.dry_run {
        DryRunPreview::new("aave", "withdraw")
            .param("chain", chain.canonical_name())
            .param("asset", &format!("{symbol} ({:#x})", asset_addr))
            .param("amount", &amount_display)
            .display();
        return Ok(());
    }

    // Confirmation.
    ctx.confirm_dangerous(&format!("withdraw {amount_display} on {chain}"))?;

    // Build wallet provider.
    let wallet_prov = heat_evm::signer::wallet_provider(ctx, chain, &rpc_url).await?;
    let user = heat_evm::signer::resolve_eoa_address(ctx)?;

    // Execute withdraw.
    ctx.output
        .diagnostic(&format!("Withdrawing {amount_display}..."));
    let tx_hash =
        crate::write::withdraw(&wallet_prov, resolved.pool, asset_addr, amount, user).await?;

    let dto = WithdrawResultDto {
        chain: chain.canonical_name().to_owned(),
        account: format!("{:#x}", user),
        asset_symbol: symbol,
        asset_address: format!("{:#x}", asset_addr),
        amount_requested: amount.to_string(),
        amount_requested_display: amount_display,
        tx_hash: format!("{:#x}", tx_hash),
    };

    ctx.output
        .write_data(&dto, Some(&pretty_withdraw))
        .map_err(io_err)
}

// ---------------------------------------------------------------------------
// Pretty formatters
// ---------------------------------------------------------------------------

fn pretty_markets(dto: &MarketsListDto) -> String {
    let mut out = format!(
        "Aave V3 Markets on {} ({} reserves)\n\n",
        dto.chain,
        dto.markets.len()
    );
    out.push_str(&format!(
        "{:<8} {:<42} {:<3} {:<8} {:<6} {:<6}\n",
        "SYMBOL", "ADDRESS", "DEC", "APY", "LTV", "FLAGS"
    ));
    out.push_str(&"-".repeat(80));
    out.push('\n');

    for m in &dto.markets {
        let flags = format!(
            "{}{}{}",
            if m.is_active { "" } else { "X" },
            if m.is_frozen { "F" } else { "" },
            if m.is_paused { "P" } else { "" },
        );
        out.push_str(&format!(
            "{:<8} {:<42} {:<3} {:<8} {:<6} {:<6}\n",
            truncate(&m.symbol, 7),
            m.underlying_address,
            m.decimals,
            format!("{}%", m.supply_apy),
            format!("{}%", m.ltv_bps as f64 / 100.0),
            flags,
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_positions(dto: &PositionsListDto) -> String {
    if dto.positions.is_empty() {
        return format!(
            "No active Aave V3 positions on {} for {}",
            dto.chain, dto.account
        );
    }

    let mut out = format!("Aave V3 Positions on {} for {}\n\n", dto.chain, dto.account);
    out.push_str(&format!(
        "{:<8} {:<18} {:<18} {:<18} {:<5}\n",
        "SYMBOL", "SUPPLIED", "STABLE DEBT", "VAR DEBT", "COLL"
    ));
    out.push_str(&"-".repeat(72));
    out.push('\n');

    for p in &dto.positions {
        let supplied_display = abbreviate_amount(&p.supplied, p.decimals);
        let stable_display = abbreviate_amount(&p.stable_debt, p.decimals);
        let variable_display = abbreviate_amount(&p.variable_debt, p.decimals);
        out.push_str(&format!(
            "{:<8} {:<18} {:<18} {:<18} {:<5}\n",
            truncate(&p.symbol, 7),
            supplied_display,
            stable_display,
            variable_display,
            if p.collateral_enabled { "yes" } else { "no" },
        ));
    }
    out.trim_end_matches('\n').to_owned()
}

fn pretty_health(dto: &HealthDto) -> String {
    format!(
        "Aave V3 Health — {} on {}\n\
         Health factor: {}\n\
         Total collateral (base): {}\n\
         Total debt (base):       {}\n\
         Available borrows:       {}\n\
         LTV:                     {}%\n\
         Liq threshold:           {}%",
        dto.account,
        dto.chain,
        dto.health_factor_display,
        dto.total_collateral_base,
        dto.total_debt_base,
        dto.available_borrows_base,
        dto.ltv_bps as f64 / 100.0,
        dto.liquidation_threshold_bps as f64 / 100.0,
    )
}

fn pretty_supply(dto: &SupplyResultDto) -> String {
    let mut out = format!(
        "Supplied {} on {}\n\
         Account: {}\n\
         Asset:   {} ({})\n\
         Tx:      {}",
        dto.amount_display,
        dto.chain,
        dto.account,
        dto.asset_symbol,
        dto.asset_address,
        dto.tx_hash,
    );
    if let Some(approval) = &dto.approval_tx_hash {
        out.push_str(&format!("\nApproval tx: {approval}"));
    }
    out
}

fn pretty_withdraw(dto: &WithdrawResultDto) -> String {
    format!(
        "Withdrew {} on {}\n\
         Account: {}\n\
         Asset:   {} ({})\n\
         Tx:      {}",
        dto.amount_requested_display,
        dto.chain,
        dto.account,
        dto.asset_symbol,
        dto.asset_address,
        dto.tx_hash,
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

/// Abbreviate a base-unit amount string for display.
fn abbreviate_amount(base_units: &str, decimals: u8) -> String {
    let Ok(val) = base_units.parse::<alloy::primitives::U256>() else {
        return "?".to_owned();
    };
    heat_evm::amount::format_units(val, decimals)
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Write failed: {e}"))
}
