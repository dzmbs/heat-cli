//! CLI branding — HEAT CLI wordmark for help output.

/// Static banner for clap's `before_help`. Uses ANSI bold white + 256-color 202 (orange-red).
/// Shown only when clap renders help text (which respects terminal context).
pub const BANNER: &str = "\
\x1b[1m\x1b[97m  HEAT\x1b[0m \x1b[38;5;202mCLI\x1b[0m
\x1b[38;5;245m  protocol-first crypto cli\x1b[0m";
