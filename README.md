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
  Trade, bridge, lend, and inspect crypto protocols from your terminal with one consistent account, safety, and output model.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.4.0-blue" />
  <img src="https://img.shields.io/badge/license-MIT-green" />
  <img src="https://img.shields.io/badge/rust-1.85%2B-orange" />
</p>

---

## Why Heat

- **One control surface.** Same flags, same output contract, same safety model across every protocol. Learn once, use everywhere.
- **Shared accounts.** One keystore across trading, lending, and bridging workflows. EVM accounts work across Hyperliquid, Polymarket, Aave, and LI.FI.
- **Reliable machine output.** Stdout is data, stderr is diagnostics. JSON, NDJSON, pretty, or quiet — same command style everywhere. Pipe to jq, feed to agents.
- **Guardrails built in.** `--dry-run` previews, TTY confirmations, `--yes` to opt in for scripts. You stay in control.
- **Protocol-first design.** Each protocol is a first-class command tree (`heat hl`, `heat pm`, `heat aave`, `heat lifi`, `heat llama`) with native vocabulary and no forced abstractions.

## Supported Protocols

| Protocol | Status | Commands |
|----------|--------|----------|
| Hyperliquid | Supported | `price` `perps` `spot` `balance` `positions` `orders` `buy` `sell` `cancel` `leverage` `send` `stream` |
| Polymarket (`heat polymarket`, alias `heat pm`) | Supported | `price` `buy` `sell` `cancel` `orders` `trades` `positions` `balance` `markets` `approve` `bridge` `ctf` `stream` |
| Aave V3 | Supported (Phase 1) | `markets` `positions` `health` `supply` `withdraw` |
| LI.FI | Supported | `chains` `tokens` `tools` `quote` `routes` `bridge` `status` |
| DefiLlama (`heat llama`, alias `heat dl`) | Supported (27 free / 15 pro) | `protocols` `chains` `coins` `stablecoins` `bridges` `fees` `volumes` `yields` `usage` |

## Quick Start

```bash
# Install (recommended)
curl -fsSL https://raw.githubusercontent.com/dzmbs/heat-cli/main/install.sh | bash

# Or install with cargo
cargo install --git https://github.com/dzmbs/heat-cli --bin heat

# Create an account
heat accounts create main --generate --persist-password ~/.heat/secrets/main.password

# Hyperliquid — check price
heat hl price ETH

# Polymarket — browse markets
heat pm markets search "election"

# Aave — inspect reserves
heat aave markets --chain ethereum

# LI.FI — preview a bridge
heat lifi bridge 100 USDC --from ethereum --to arbitrum --account main --dry-run

# DefiLlama — top protocols by TVL
heat llama protocols list --limit 5

# Machine-readable output
heat hl balance --json | jq '.balances'
```

## Example Workflows

### Hyperliquid

```bash
heat hl price BTC
heat hl buy ETH 0.5 --price 3500 --dry-run
heat hl stream trades BTC
```

### Polymarket

```bash
heat pm markets search "election"
heat pm buy <token_id> --price 0.55 --size 100 --dry-run
heat pm positions --json
```

### Aave

```bash
heat aave markets --chain ethereum
heat aave health --chain base --account main
heat aave supply USDC 1000 --chain arbitrum --account main --dry-run
```

### LI.FI

```bash
heat lifi routes --from-chain ethereum --to-chain base --from-token USDC --to-token USDC --amount 1000000 --from-address 0x...
heat lifi bridge 100 USDC --from ethereum --to arbitrum --account main --dry-run
heat lifi status --tx-hash 0x... --from-chain ethereum --to-chain arbitrum --bridge stargate
```

### DefiLlama

```bash
heat llama protocols list --sort tvl --limit 10
heat llama protocols get aave --json
heat llama coins price "coingecko:bitcoin,coingecko:ethereum"
heat llama fees overview --chain Ethereum
heat llama yields pools --chain Ethereum --sort apy --limit 10
```

## Output Modes

Heat auto-detects your environment and picks the right format.

| Mode | When | Example |
|------|------|---------|
| **pretty** | TTY (default) | `heat hl price BTC` |
| **json** | Piped (default), or `--json` | `heat aave markets --chain ethereum --json` |
| **ndjson** | `--output ndjson` | `heat hl stream trades BTC` |
| **quiet** | `-q` / `--quiet` | `HF=$(heat aave health --chain base -q)` |

## Safety

Heat handles real money. It ships with guardrails:

- **`--dry-run`** previews what a command would do without executing it.
- **TTY confirmation** prompts before dangerous writes (orders, sends, approvals, bridge execution, lending actions).
- **`--yes`** skips the prompt for non-interactive scripts.
- **Non-TTY without `--yes`** fails with an error. Scripts must opt in explicitly.

See [Safety](https://heat-cli.vercel.app/core/safety) for details.

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

See [Installation](https://heat-cli.vercel.app/introduction/installation) for platform details and release downloads.

## Current Limitations

Heat v0.4.0 is an early but real release. Current boundaries:

- Aave currently supports `markets`, `positions`, `health`, `supply`, and `withdraw` — not `borrow`/`repay` yet.
- LI.FI currently focuses on supported EVM routing/bridging flows — not non-EVM execution.
- Local key accounts only — no hardware wallet or multisig.
- Use with small amounts until you trust it in your environment.
- Polymarket API key commands are still limited by an upstream SDK issue.

See [Limitations](https://heat-cli.vercel.app/reference/limitations) for the full list.

## Documentation

- [Getting Started](https://heat-cli.vercel.app/introduction/getting-started)
- [Installation](https://heat-cli.vercel.app/introduction/installation)
- [Accounts](https://heat-cli.vercel.app/core/accounts)
- [Output Modes](https://heat-cli.vercel.app/core/output)
- [Safety](https://heat-cli.vercel.app/core/safety)
- [Hyperliquid Overview](https://heat-cli.vercel.app/protocols/hyperliquid)
- [Hyperliquid Onboarding](https://heat-cli.vercel.app/protocols/hyperliquid-onboarding)
- [Polymarket Overview](https://heat-cli.vercel.app/protocols/polymarket)
- [Polymarket Onboarding](https://heat-cli.vercel.app/protocols/polymarket-onboarding)
- [Aave Overview](https://heat-cli.vercel.app/protocols/aave)
- [LI.FI Overview](https://heat-cli.vercel.app/protocols/lifi)
- [DefiLlama Overview](https://heat-cli.vercel.app/protocols/defillama)
- [Limitations](https://heat-cli.vercel.app/reference/limitations)

## License

MIT
