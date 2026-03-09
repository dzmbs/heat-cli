use clap::{Parser, Subcommand};
use heat_core::config::HeatConfig;
use heat_core::ctx::Ctx;
use heat_core::error::HeatError;
use heat_core::output::OutputFormat;

mod cmd_accounts;
mod cmd_wallet;

#[derive(Parser)]
#[command(name = "heat", about = "Unified crypto CLI for humans and agents")]
#[command(version, propagate_version = true)]
#[command(before_help = heat_core::branding::BANNER)]
struct Cli {
    /// Output raw JSON (shorthand for --output json)
    #[arg(long, global = true)]
    json: bool,

    /// Output format: pretty, json, ndjson, quiet
    #[arg(long, global = true)]
    output: Option<String>,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Account to use
    #[arg(long, global = true)]
    account: Option<String>,

    /// Network override
    #[arg(long, global = true)]
    network: Option<String>,

    /// Preview action without executing
    #[arg(long, global = true)]
    dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long, global = true)]
    yes: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Aave V3 lending protocol commands
    Aave(heat_aave::cmd::AaveCmd),

    /// Manage accounts
    Accounts(cmd_accounts::AccountsCmd),

    /// Hyperliquid protocol commands
    Hl(heat_hl::cmd::HlCmd),

    /// LI.FI cross-chain bridge and swap commands
    Lifi(heat_lifi::cmd::LifiCmd),

    /// Polymarket protocol commands
    #[command(alias = "pm")]
    Polymarket(heat_pm::cmd::PmCmd),

    /// Wallet operations (balances, cross-chain views)
    Wallet(cmd_wallet::WalletCmd),

    /// Show current configuration
    Config,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let format = match OutputFormat::auto_detect(cli.json, cli.output.as_deref(), cli.quiet) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit();
        }
    };

    let config = match HeatConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit();
        }
    };

    let ctx = match Ctx::new(
        format,
        config,
        cli.account,
        cli.network,
        cli.dry_run,
        cli.yes,
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit();
        }
    };

    let result = run(cli.command, &ctx).await;

    if let Err(e) = result {
        let _ = ctx.output.write_error(&e);
        e.exit();
    }
}

async fn run(command: Command, ctx: &Ctx) -> Result<(), HeatError> {
    match command {
        Command::Aave(cmd) => heat_aave::cmd::run(cmd, ctx).await,
        Command::Accounts(cmd) => cmd_accounts::run(cmd, ctx),
        Command::Hl(cmd) => heat_hl::cmd::run(cmd, ctx).await,
        Command::Lifi(cmd) => heat_lifi::cmd::run(cmd, ctx).await,
        Command::Polymarket(cmd) => heat_pm::cmd::run(cmd, ctx).await,
        Command::Wallet(cmd) => cmd_wallet::run(cmd, ctx).await,
        Command::Config => cmd_config(ctx),
    }
}

fn cmd_config(ctx: &Ctx) -> Result<(), HeatError> {
    let home = HeatConfig::home_dir()?;
    let config_path = home.join("config.toml");

    if ctx.output.format == OutputFormat::Json {
        ctx.output.write_data(&ctx.config, None).map_err(io_err)
    } else {
        println!("Config file: {}", config_path.display());
        println!("Heat home:   {}", home.display());
        if let Some(acc) = &ctx.config.default_account {
            println!("Default account: {acc}");
        }
        if let Some(net) = &ctx.config.network {
            println!("Default network: {net}");
        }
        if let Some(out) = &ctx.config.output {
            println!("Default output:  {out}");
        }
        if let Some(acc) = &ctx.account_name {
            println!("Resolved account: {acc}");
        }
        if let Some(net) = &ctx.network {
            println!("Resolved network: {net}");
        }
        if ctx.config.protocols.is_empty() {
            println!("No protocol-specific config.");
        } else {
            for (proto, val) in &ctx.config.protocols {
                println!("[{proto}] {val}");
            }
        }
        Ok(())
    }
}

fn io_err(e: std::io::Error) -> HeatError {
    HeatError::internal("output", format!("Failed to write output: {e}"))
}
