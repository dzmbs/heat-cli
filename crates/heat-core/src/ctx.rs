use crate::accounts::{self, Account};
use crate::config::HeatConfig;
use crate::error::{ErrorCategory, HeatError};
use crate::output::{Output, OutputFormat};

/// Shared execution context — resolved once at startup, passed to all commands.
pub struct Ctx {
    pub output: Output,
    pub config: HeatConfig,
    pub account_name: Option<String>,
    pub network: Option<String>,
    pub dry_run: bool,
    pub yes: bool,
}

impl Ctx {
    pub fn new(
        format: OutputFormat,
        config: HeatConfig,
        account_flag: Option<String>,
        network_flag: Option<String>,
        dry_run: bool,
        yes: bool,
    ) -> Result<Self, HeatError> {
        // Resolve account: flag > env > config > default-account file
        // Only suppress "no account configured" — propagate real errors.
        let account_name = match accounts::resolve_account_name(
            account_flag.as_deref(),
            &config,
        ) {
            Ok(name) => Some(name),
            Err(e) if e.category == ErrorCategory::Auth && e.reason == "no_account" => None,
            Err(e) => return Err(e),
        };

        // Resolve network: flag > env > config > None
        let network = network_flag
            .or_else(|| std::env::var("HEAT_NETWORK").ok().filter(|s| !s.is_empty()))
            .or_else(|| config.network.clone());

        Ok(Self {
            output: Output::new(format),
            config,
            account_name,
            network,
            dry_run,
            yes,
        })
    }

    /// Get the resolved account. Errors if no account is available.
    pub fn require_account(&self) -> Result<Account, HeatError> {
        let name = self.account_name.as_deref().ok_or_else(|| {
            HeatError::auth("no_account", "No account specified")
                .with_hint("Use --account <NAME>, set HEAT_ACCOUNT, or run 'heat accounts use <NAME>'")
        })?;
        Account::load(name)
    }

    /// Confirm a dangerous action using the resolved safety flags.
    pub fn confirm_dangerous(&self, action: &str) -> Result<(), HeatError> {
        crate::safety::confirm_dangerous(action, self.yes, self.output.is_tty)
    }
}
