# Heat CLI - Roadmap

This roadmap turns `PLAN.md` into a practical execution sequence.

It is meant to answer:
- what we build now
- what we defer
- what counts as done
- what comes next

The roadmap should stay stable even if implementation details change.

---

## Product Direction

Heat is a unified crypto CLI for:
- traders
- AI agents
- researchers
- terminal power users

Heat is:
- protocol-first at the top level
- helper-first in UX
- shared-core underneath
- safety-aware for real money actions
- machine-readable in automation

Initial protocol order:
1. Hyperliquid
2. Polymarket
3. LI.FI
4. Aave / Kamino / Pacific class protocols

---

## Roadmap Structure

Work is split into:
- **Now**: must be built before Heat is real
- **Next**: should happen after the first strong release shape exists
- **Later**: important but intentionally deferred
- **Explicit non-goals for now**: do not spend time here yet

---

# NOW

## Phase 0 - Foundation [COMPLETE]

### Objective
Build the shared product contract for Heat before protocol count grows.

### Why this phase matters
If the foundation is wrong, every new protocol will bolt on a different:
- output system
- account flow
- config model
- safety behavior
- error shape

That is exactly what Heat should avoid.

### Scope

#### CLI behavior
- protocol-first top-level command shape
- global flags for output, account, network, dry-run, confirmation
- stable stdout / stderr contract
- stable exit code contract

#### Output
- `pretty` for TTY
- `json` for non-TTY / `--json`
- `ndjson` for streams
- `quiet` for scalar outputs where sensible

#### Errors
- shared error categories
- structured machine-readable errors
- stable exit codes

#### Accounts
- account-first UX
- EVM-backed accounts for phase 1
- encrypted keystore support
- account selection and default account behavior
- raw key escape hatch, but not as primary UX

#### Config
- layered precedence
- global + protocol-specific config
- default account / output / network

#### Safety
- `--dry-run`
- dangerous-write confirmation in TTY unless `--yes`
- no prompts in non-TTY

#### Validation
- shared validation helpers
- command input validation before execution
- file write safety / path safety

#### Project hygiene
- tests for output contracts
- tests for account/config precedence
- tests for keystore roundtrip
- roadmap tracking docs

### Deliverables
- first complete Heat CLI shell with foundation behavior
- documented and tested shared contracts
- enough core in place to implement Hyperliquid cleanly

### Exit criteria
Phase 0 is done when:
- top-level Heat binary exists and runs
- output modes work as designed
- account resolution works deterministically
- dry-run and confirmation rules are implemented
- config precedence is implemented and tested
- stdout/stderr behavior is documented and tested
- developers can start Hyperliquid without redefining core behavior

---

## Phase 1 - Hyperliquid [COMPLETE]

### Objective
Ship the first real protocol integration and prove the Heat model works.

### Implementation policy
- use official Rust dependencies (`hypersdk`)
- adapt production-proven Hyperliquid CLI logic where useful
- do not rewrite protocol behavior from scratch unless necessary
- keep Heat-native output / account / config / safety behavior

### Scope

#### Read/query commands
- price
- perps
- spot
- balance
- positions
- orders

#### Write commands
- buy
- sell
- cancel
- leverage
- send

#### Streams
- trades stream first

### UX goals
- helper-first command surface
- good pretty output on TTY
- clean JSON for automation
- NDJSON for stream output

### Safety goals
- dry-run support where practical
- confirmation behavior for dangerous writes
- protocol validation before order submission

### Deliverables
- a usable Hyperliquid experience inside Heat
- enough confidence to say the shared core works for one trading protocol

### Exit criteria
Phase 1 is done when:
- a trader can use Heat for practical Hyperliquid workflows
- an agent can call Hyperliquid commands and parse outputs reliably
- Hyperliquid-specific logic lives behind Heat’s shared contracts
- no protocol-specific output/account/config hacks are leaking into the core

---

## Phase 2 - Polymarket [COMPLETE]

### Objective
Prove the Heat model works for a second protocol with meaningfully different auth and command shape.

### Implementation policy
- use official client dependencies (`polymarket-client-sdk 0.4.3`)
- adapt proven `polymarket-cli` logic where useful
- keep Heat-native output / accounts / config / safety contracts

### Scope

#### Helper-first commands (top-level shortcuts)
- price, buy, sell, cancel, orders, trades, positions, balance

#### Protocol-native trees
- gamma (markets list/get/search, events, comments)
- clob (limit-order, cancel, open-orders, trades, book, markets, balance, rewards, earnings)
- data (positions, trades, activity, holders, volume, open-interest, leaderboard)
- bridge (deposit addresses, status, supported-assets)
- ctf (split, merge, redeem, collection-id)
- approve (check, set)

### Status
Accepted as complete enough to move on.

Implemented:
- helper commands
- Gamma/CLOB/Data/Bridge/CTF/Approve trees
- Polymarket account/signature-type handling
- structured Heat-owned JSON output
- test coverage across auth, bridge, data, CLOB, and crate integration
- real-time WebSocket streaming for:
  - market data: `stream orderbook`, `stream prices`, `stream midpoints`
  - user events: `stream orders`, `stream trades`

Remaining hardening work:
- more integration tests
- more manual mainnet testing of write paths
- ApiKeys remains limited by upstream SDK response access

### Special goals
- [x] prove account model can support protocol-specific signer/auth settings
- [x] prove protocol differences can be handled without breaking the shared core

### Deliverables
- practical Polymarket support in Heat
- stronger confidence in account/config/output architecture

### Exit criteria
Phase 2 is done when:
- both Hyperliquid and Polymarket feel like the same product
- while still respecting protocol differences
- and without duplicating core infrastructure

---

# NEXT

## Shared EVM substrate [COMPLETE]

### Objective
Extract the reusable EVM pieces now that Heat supports multiple EVM-native and EVM-adjacent protocols.

### Delivered
- `heat-evm` crate with canonical chain model (Ethereum, Polygon, Arbitrum, Optimism, Base)
- reusable signer + provider helpers built on Heat accounts
- RPC resolution with config/env precedence
- ERC-20 approval/allowance helpers
- amount parsing/formatting
- `EvmChain::from_chain_id()` for execution boundary checks
- used by both heat-aave and heat-lifi

## Phase 3 - LI.FI [COMPLETE]

### Objective
Add EVM route/bridge integration as Heat's capital-movement layer.

### Implementation policy
- direct REST integration against LI.FI API (`https://li.quest/v1/`)
- heat-evm for execution substrate (wallet, RPC, ERC-20)
- Heat-native output, safety, accounts, chain naming

### Supported scope
Chains:
- Ethereum, Polygon, Arbitrum, Base, Optimism

Commands:
- `heat lifi chains` / `tokens` / `tools` — discovery (accepts any LI.FI chain)
- `heat lifi quote` / `routes` — read (accepts any LI.FI chain)
- `heat lifi bridge` — execution (Heat-supported EVM chains only)
- `heat lifi status` — transfer tracking

Execution:
- EVM-only routes where all chains are in Heat's supported set
- ERC-20 approval with allowance checking
- native token detection (zero address + LI.FI placeholder)
- `action.fromAddress` injection for stepTransaction
- gas limit parsing handles hex and decimal formats

Safety:
- `--dry-run` preview with route details
- TTY confirmation via `confirm_dangerous`
- `--yes` override for non-interactive use
- execution classification truthfully matches actual capability

### Explicitly not supported
- non-EVM route execution (Solana, Cosmos, etc.)
- EVM chains outside Heat's supported set
- HyperCore
- broad "all chains / all tokens" claims

### Status
Accepted after adversarial review. Two blockers fixed:
1. `action.fromAddress` injection into stepTransaction requests
2. execution classification tightened to Heat-supported chains only

---

## Phase 4 - Lending / strategy protocol class

### Objective
Extend Heat into lending / collateral / position-management workflows.

### Candidate protocols
- Aave
- Kamino
- Pacific
- DefiLlama

### Implementation policy
Treat each protocol as a protocol tree.
Do not force a universal `lend` tree yet.

Before implementation, do a short protocol interview to decide:
- official SDK/client source
- signer/account needs
- helper commands vs native commands
- must-have features

### Likely first-scope actions
- supply / deposit
- withdraw
- borrow
- repay
- positions
- markets / reserves

### Goals
- prove Heat can extend beyond trading and bridging
- pressure-test the account model for non-EVM-only future support

---

## Phase 5 - Research / intelligence data layer

### Objective
Add a broad read-only protocol/data integration layer for market intelligence and research workflows.

### Candidate protocols
- DefiLlama

### Why this phase matters
Heat already covers execution-heavy protocol workflows.
A serious research/data protocol makes Heat more useful for:
- agents
- researchers
- discretionary traders
- terminal users doing discovery and monitoring

### DefiLlama implementation policy
- treat DefiLlama as a protocol-first data tree (`heat llama ...`)
- use direct REST integration in Rust
- support public endpoints first, but design for pro-key support from day one
- keep Heat-native output DTOs and stable machine-readable contracts
- do not build around website-only subscription/dashboard features

### Likely first-scope actions
- protocols / TVL
- chains
- coins / prices
- stablecoins
- bridges
- fees / DEX volumes

### Follow-on scope
- yields / borrow / perps / LSD rates
- unlocks / emissions
- categories / forks / oracles / entities
- raises / treasuries / hacks
- institutions / ETFs / FDV / usage

---

# LATER

These matter, but should come after the main protocol foundation is proven.

## Product improvements
- optional envelope mode
- richer dry-run previews
- command introspection / explain tooling
- generated docs from command metadata
- examples / cookbook docs per protocol

## Account improvements
- Solana-backed accounts
- richer per-account metadata
- protocol-specific account capabilities

## CLI quality
- shell completion
- release/install polish
- stronger snapshot testing of outputs

## Architecture improvements
- cleaner extension / plugin story
- protocol metadata registry if truly needed later
- shared retry / backoff utilities if repeated enough to justify centralization

---

# EXPLICIT NON-GOALS FOR NOW

Do not spend roadmap energy here yet:

- hardware wallets
- multisig support
- full plugin SDK
- TUI
- portfolio analytics
- strategy automation
- background daemons

These are not forbidden forever.
They are simply not roadmap priorities yet.

---

# Protocol Intake Rule

Before any new protocol is implemented:
1. do a short design interview with the product owner
2. decide what to reuse vs rewrite
3. confirm must-have scope for first integration
4. confirm what is deferred

Questions to answer before starting:
- what official SDK/client should be used?
- what upstream CLI / package logic should be reused directly?
- what should only be used as reference?
- what Heat commands should exist in v1 for this protocol?
- what signer/account behavior is required?
- what dry-run/safety behavior is required?
- what is out of scope for first pass?

This is a required step, not optional process overhead.

---

# Definition of a Good Heat Release

A Heat release is good if:
- humans can use it confidently in a terminal
- agents can parse it reliably in automation
- dangerous actions are safer than ad hoc scripts
- protocols feel consistent without feeling fake-unified
- new protocols can be added without redesigning the product each time

---

# Working Rule for the Team

When in doubt:
- reuse mature protocol logic
- standardize Heat UX/contracts
- ship the smallest strong version first
- do not overengineer future extensibility
