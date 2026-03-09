use clap::{Args, Subcommand};
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;
use heat_evm::balance::{self, TokenSpec};
use serde::Serialize;

#[derive(Args)]
pub struct WalletCmd {
    #[command(subcommand)]
    pub command: WalletSubcommand,
}

#[derive(Subcommand)]
pub enum WalletSubcommand {
    /// Show wallet balances across EVM chains
    Balance {
        /// Comma-separated chains (e.g. ethereum,base,arbitrum)
        #[arg(long, default_value = "ethereum,polygon,arbitrum,optimism,base")]
        chains: String,
        /// Comma-separated tokens: native, USDC, USDT, WETH, DAI, or 0x address
        #[arg(long, default_value = "native")]
        tokens: String,
        /// RPC URL override (applies to all chains — use env vars for per-chain overrides)
        #[arg(long)]
        rpc: Option<String>,
    },
}

#[derive(Serialize)]
struct BalanceEntry {
    chain: String,
    chain_id: u64,
    token_symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_address: Option<String>,
    decimals: u8,
    amount: String,
    amount_display: String,
    token_type: String,
}

#[derive(Serialize)]
struct WalletBalanceOutput {
    account: String,
    address: String,
    balances: Vec<BalanceEntry>,
}

pub async fn run(cmd: WalletCmd, ctx: &Ctx) -> Result<(), HeatError> {
    match cmd.command {
        WalletSubcommand::Balance {
            chains,
            tokens,
            rpc,
        } => cmd_balance(&chains, &tokens, rpc.as_deref(), ctx).await,
    }
}

async fn cmd_balance(
    chains_input: &str,
    tokens_input: &str,
    rpc_override: Option<&str>,
    ctx: &Ctx,
) -> Result<(), HeatError> {
    let chains = balance::parse_chains(chains_input)?;

    let address = heat_evm::resolve_eoa_address(ctx)?;
    let account_name = ctx.account_name.as_deref().unwrap_or("unknown").to_string();

    let mut entries = Vec::new();

    for chain in &chains {
        let tokens = balance::parse_tokens(tokens_input, *chain)?;
        let rpc_url = heat_evm::rpc::resolve_rpc_url(ctx, *chain, rpc_override, None)?;
        let provider = heat_evm::read_provider(*chain, &rpc_url).await?;

        for token in &tokens {
            let entry = match token {
                TokenSpec::Native => {
                    let bal = balance::native_balance(provider.clone(), address).await?;
                    let decimals = 18u8;
                    let display = heat_evm::amount::format_units(bal, decimals);
                    BalanceEntry {
                        chain: chain.canonical_name().to_string(),
                        chain_id: chain.chain_id(),
                        token_symbol: chain.native_symbol().to_string(),
                        token_address: None,
                        decimals,
                        amount: bal.to_string(),
                        amount_display: display,
                        token_type: "native".to_string(),
                    }
                }
                TokenSpec::Erc20 {
                    address: token_addr,
                    known_symbol,
                    known_decimals,
                } => {
                    let bal =
                        heat_evm::erc20::balance_of(provider.clone(), *token_addr, address).await?;
                    let (symbol, decimals) = match (known_symbol, known_decimals) {
                        (Some(s), Some(d)) => (s.clone(), *d),
                        _ => {
                            let s = heat_evm::erc20::symbol(provider.clone(), *token_addr).await?;
                            let d =
                                heat_evm::erc20::decimals(provider.clone(), *token_addr).await?;
                            (s, d)
                        }
                    };
                    let display = heat_evm::amount::format_units(bal, decimals);
                    BalanceEntry {
                        chain: chain.canonical_name().to_string(),
                        chain_id: chain.chain_id(),
                        token_symbol: symbol,
                        token_address: Some(format!("{token_addr:#x}")),
                        decimals,
                        amount: bal.to_string(),
                        amount_display: display,
                        token_type: "erc20".to_string(),
                    }
                }
            };
            entries.push(entry);
        }
    }

    let output = WalletBalanceOutput {
        account: account_name,
        address: format!("{address:#x}"),
        balances: entries,
    };

    match ctx.output.format {
        OutputFormat::Pretty => {
            println!("Account: {} ({})", output.account, output.address);
            println!();
            if output.balances.is_empty() {
                println!("  No balances found.");
            } else {
                println!("  {:<12} {:<8} {:>24}  Type", "Chain", "Token", "Balance");
                println!("  {}", "-".repeat(60));
                for b in &output.balances {
                    println!(
                        "  {:<12} {:<8} {:>24}  {}",
                        b.chain, b.token_symbol, b.amount_display, b.token_type
                    );
                }
            }
        }
        OutputFormat::Json | OutputFormat::Ndjson => {
            ctx.output.write_data(&output, None).map_err(|e| {
                HeatError::internal("output", format!("Failed to write output: {e}"))
            })?;
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}
