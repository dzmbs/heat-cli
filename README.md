<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./logo.svg" />
    <source media="(prefers-color-scheme: light)" srcset="./logo-dark.svg" />
    <img src="./logo-dark.svg" alt="Heat CLI" width="280" />
  </picture>
</p>

<p align="center">
  <strong>One CLI for all of finance — built for humans and AI agents.</strong>
</p>
<p align="center">
  Trade, inspect, and automate Hyperliquid and Polymarket from your terminal. For humans and AI agents.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.0-blue" />
  <img src="https://img.shields.io/badge/license-MIT-green" />
  <img src="https://img.shields.io/badge/rust-1.92%2B-orange" />
</p>

---

## Why Heat

- **One control surface.** Same flags, same output contract, same safety model across every protocol. Learn once, use everywhere.
- **Shared accounts.** One keystore across all protocols. EVM accounts work across Hyperliquid and Polymarket.
- **Reliable machine output.** Stdout is data, stderr is diagnostics. JSON, NDJSON, pretty, or quiet — same commands everywhere. Pipe to jq, feed to agents.
- **Guardrails built in.** `--dry-run` previews, TTY confirmations, `--yes` to opt in for scripts. You stay in control.
- **Protocol-first design.** Each protocol is a first-class command tree (`heat hl`, `heat pm`) with native vocabulary and no forced abstractions.

## Supported Protocols

| Protocol | Status | Commands |
|----------|--------|----------|
| Hyperliquid | Supported | `price` `perps` `spot` `balance` `positions` `orders` `buy` `sell` `cancel` `leverage` `send` `stream` |
| Polymarket | Supported | `price` `buy` `sell` `cancel` `orders` `trades` `positions` `balance` `markets` `approve` `bridge` `ctf` |
| LI.FI | Coming next | -- |

## Quick Start

```bash
# Install (recommended)
curl -fsSL https://raw.githubusercontent.com/dzmbs/heat-cli/main/install.sh | bash

# Or install with cargo
cargo install --git https://github.com/dzmbs/heat-cli --bin heat

# Create an account
heat accounts create myaccount

# Check ETH price on Hyperliquid
heat hl price ETH

# Browse Polymarket markets
heat pm markets search "election"

# Machine-readable output
heat hl balance --json | jq '.balances'
```

## Output Modes

Heat auto-detects your environment and picks the right format.

| Mode | When | Example |
|------|------|---------|
| **pretty** | TTY (default) | `heat hl price BTC` |
| **json** | Piped (default), or `--json` | `heat hl price BTC \| jq .mid` |
| **ndjson** | `--output ndjson` | `heat hl stream trades BTC` |
| **quiet** | `-q` / `--quiet` | `PRICE=$(heat hl price BTC -q)` |

## Safety

Heat handles real money. It ships with guardrails:

- **`--dry-run`** previews what a command would do without executing it.
- **TTY confirmation** prompts before dangerous writes (orders, sends, approvals).
- **`--yes`** skips the prompt for non-interactive scripts.
- **Non-TTY without `--yes`** fails with an error. Scripts must opt in explicitly.

See [Safety](https://dzmbs.github.io/heat-cli/core/safety) for details.

## Installation

```bash
# One-command install (recommended)
curl -fsSL https://raw.githubusercontent.com/dzmbs/heat-cli/main/install.sh | bash

# Or install with cargo
cargo install --git https://github.com/dzmbs/heat-cli --bin heat

# Or from a local checkout
cargo install --path crates/heat-cli

# Verify
heat --version
```

See [Installation](https://dzmbs.github.io/heat-cli/introduction/installation) for platform details and release downloads.

## Current Limitations

Heat v0.1.0 is an early release. Be honest with yourself:

- No built-in bridging yet (LI.FI planned)
- Local key accounts only -- no hardware wallet or multisig
- Use with small amounts until you trust it
- Polymarket API key commands are not yet functional (upstream SDK limitation)

See [Limitations](https://dzmbs.github.io/heat-cli/reference/limitations) for the full list.

## Documentation

- [Getting Started](https://dzmbs.github.io/heat-cli/introduction/getting-started)
- [Installation](https://dzmbs.github.io/heat-cli/introduction/installation)
- [Accounts](https://dzmbs.github.io/heat-cli/core/accounts)
- [Output Modes](https://dzmbs.github.io/heat-cli/core/output)
- [Safety](https://dzmbs.github.io/heat-cli/core/safety)
- [Hyperliquid](https://dzmbs.github.io/heat-cli/protocols/hyperliquid)
- [Polymarket](https://dzmbs.github.io/heat-cli/protocols/polymarket)
- [Limitations](https://dzmbs.github.io/heat-cli/reference/limitations)

## License

MIT
