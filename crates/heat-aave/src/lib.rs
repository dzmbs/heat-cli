/// heat-aave — Aave V3 protocol integration for the Heat CLI.
///
/// Phase 1 scope: read-only market/position/health queries and
/// supply/withdraw write operations on Ethereum, Arbitrum, and Base.
///
/// # Architecture
/// - `addresses` — static Aave V3 market registry per chain (for tests/validation)
/// - `resolver`  — runtime address resolution via PoolAddressesProvider
/// - `contracts` — minimal Alloy `sol!` interfaces (only methods we call)
/// - `read`      — on-chain read operations (markets, positions, health)
/// - `write`     — on-chain write operations (supply, withdraw)
/// - `dto`       — Heat-owned stable output types
/// - `cmd`       — clap command tree (`heat aave …`)
///
/// # Contract strategy
/// - `PoolAddressesProvider` is the canonical market entrypoint;
///   Pool and DataProvider addresses are resolved from it at runtime
/// - `AaveProtocolDataProvider` for reserve config, data, and user positions
/// - `Pool` for writes and `getUserAccountData`
pub mod addresses;
pub mod cmd;
pub mod contracts;
pub mod dto;
pub mod read;
pub mod resolver;
pub mod write;

#[cfg(test)]
mod tests;
