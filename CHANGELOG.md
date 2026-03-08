# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-08

### Added
- Initial release
- Hyperliquid protocol support (price, perps, spot, balance, positions, orders, buy, sell, cancel, leverage, send, stream trades)
- Polymarket protocol support (markets, CLOB trading, data queries, bridge, CTF, approvals)
- Account management with EVM keystore (Ethereum V3)
- Four output modes: pretty, json, ndjson, quiet
- Auto-detection: TTY→pretty, pipe→json
- Safety features: --dry-run, TTY confirmations, --yes override
- Layered config system (flags > env > config > defaults)

### Known Limitations
- No built-in bridge/routing (planned)
- Local key accounts only (no hardware wallets)
- Polymarket ApiKeys command blocked on upstream SDK
