/// heat-lifi — LI.FI protocol integration for the Heat CLI.
///
/// Provides route discovery, cross-chain bridge execution, and transfer
/// status tracking via the LI.FI REST API.
///
/// # Architecture
/// - `client` — typed HTTP wrapper around `https://li.quest/v1/`
/// - `dto`    — Heat-owned stable output types (the public contract)
/// - `map`    — raw API response → DTO mapping (internal)
/// - `exec`   — execution capability classification
/// - `cmd`    — clap command tree (`heat lifi …`)
///
/// # Execution
/// - EVM → EVM routes are executed via `heat-evm` (wallet provider, ERC-20 approvals)
/// - Route steps are executed by fetching transaction data from LI.FI's
///   `/advanced/stepTransaction` endpoint and sending through Heat's signer
/// - Non-EVM routes are classified as unsupported and rejected
pub mod client;
pub mod cmd;
pub mod dto;
pub mod exec;
pub mod map;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod http_tests;
