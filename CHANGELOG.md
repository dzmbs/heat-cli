# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-03-09

### Added
- Hyperliquid support as a practical production-shaped protocol integration:
  - `price`, `perps`, `spot`, `balance`, `positions`, `orders`
  - `buy`, `sell`, `cancel`, `leverage`, `send`
  - `stream trades`
- Polymarket support as a practical production-shaped protocol integration:
  - helper-first commands for trading and account views
  - Gamma, CLOB, Data, Bridge, CTF, and Approve trees
  - WebSocket streams for market and user events
- Aave V3 phase-1 support:
  - `markets`, `positions`, `health`, `supply`, `withdraw`
  - provider-first runtime resolution through `PoolAddressesProvider`
  - Ethereum, Arbitrum, and Base support
- LI.FI support:
  - `chains`, `tokens`, `tools`, `quote`, `routes`, `bridge`, `status`
  - supported EVM bridge execution through Heat accounts and `heat-evm`
  - route discovery and execution classification
- Shared EVM substrate (`heat-evm`):
  - canonical chain naming and IDs
  - RPC resolution and validation
  - signer/provider helpers
  - exact amount parsing and formatting
  - ERC-20 approval helpers
- Shared account/config/output/safety model across all current protocols
- CI workflows for check, test, clippy, fmt, and docs build
- Public docs site coverage for Hyperliquid, Polymarket, Aave, and LI.FI

### Changed
- Heat now ships as a broader multi-protocol release centered on:
  - trading (Hyperliquid, Polymarket)
  - lending (Aave phase 1)
  - EVM capital movement (LI.FI)
- README and docs updated to reflect the real current protocol surface
- Release/install metadata updated for `0.2.0`

### Known Limitations
- Aave currently supports supply/withdraw workflows, not borrow/repay yet
- LI.FI currently targets supported EVM routing/bridging flows, not non-EVM execution
- Hyperliquid funding remains external/manual
- Polymarket funding remains partly external/manual
- Local key accounts only (no hardware wallets)
- Polymarket ApiKeys command remains limited by an upstream SDK issue

## [0.1.0] - 2026-03-08

### Added
- Initial public Heat release shape
- Hyperliquid protocol support
- Polymarket protocol support
- Account management with encrypted EVM keystore (Ethereum V3)
- Four output modes: pretty, json, ndjson, quiet
- Auto-detection: TTY → pretty, pipe → json
- Safety features: `--dry-run`, TTY confirmations, `--yes` override
- Layered config system (flags > env > config > defaults)
