# Heat CLI - Execution Checklist

This checklist is the practical companion to `PLAN.md` and `ROADMAP.md`.

Use it to track actual build progress.

---

# Phase 0 - Foundation

## 0.1 Project setup
- [x] Confirm product direction matches `PLAN.md`
- [x] Confirm roadmap order matches `ROADMAP.md`
- [x] Create Cargo workspace structure
- [x] Create top-level docs: `PLAN.md`, `ROADMAP.md`, `CHECKLIST.md`, `UPSTREAM.md`
- [ ] Add basic CI command targets (`build`, `test`, lint target if desired)

## 0.2 CLI shell
- [x] Create top-level `heat` binary
- [x] Add top-level protocol-first dispatch shape
- [x] Add global flags:
  - [x] `--json`
  - [x] `--output`
  - [x] `--quiet`
  - [x] `--account`
  - [x] `--network`
  - [x] `--dry-run`
  - [x] `--yes`
- [x] Define stable exit code mapping
- [x] Add top-level help text

## 0.3 Output contract
- [x] Implement `pretty` output mode
- [x] Implement `json` output mode
- [x] Implement `ndjson` output mode
- [x] Implement `quiet` output behavior where meaningful
- [x] Implement TTY vs non-TTY auto-detection
- [x] Ensure stdout = data only
- [x] Ensure stderr = diagnostics only
- [ ] Document output contract in code/tests/docs

## 0.4 Error contract
- [x] Define shared error categories:
  - [x] validation
  - [x] auth
  - [x] network
  - [x] protocol
  - [x] internal
- [x] Implement machine-readable JSON error output
- [x] Map errors to stable exit codes
- [x] Ensure TTY rendering remains friendly without breaking machine mode

## 0.5 Accounts and keystore
- [x] Define account-first user model
- [x] Implement account metadata storage
- [x] Implement default account support
- [x] Implement EVM keystore storage for phase 1
- [x] Implement account resolution precedence
- [x] Keep raw key as escape hatch, not primary path
- [x] Add account listing / inspect / default-selection commands
- [x] Cache EVM address in account metadata (no password for read-only)
- [x] Store address in V3 keystore (lowercase hex, no 0x prefix)
- [x] Normalize imported keystore addresses
- [x] Backfill address on first signer derivation

## 0.6 Config
- [x] Implement layered config loading
- [x] Support global config values:
  - [x] default account
  - [x] default network
  - [x] default output mode
- [x] Support protocol-specific config sections
- [x] Test precedence: flags > env > config > defaults

## 0.7 Safety
- [x] Implement `--dry-run` shared behavior
- [x] Implement dangerous-write confirmation on TTY
- [x] Implement `--yes` override
- [x] Ensure non-TTY never prompts interactively
- [x] Define which command classes are dangerous by default

## 0.8 Validation
- [x] Add shared validation helpers
- [x] Validate account existence
- [x] Validate network values
- [x] Validate numeric input parsing
- [ ] Validate local file/path safety where needed
- [x] Add a place for protocol-specific validation hooks

## 0.9 File safety / persistence
- [x] Implement atomic file writes for config/account state
- [x] Set secure permissions where appropriate
- [x] Ensure keystore/account writes are crash-safe enough

## 0.10 Tests
- [ ] Test TTY vs non-TTY output selection
- [ ] Test stdout/stderr behavior
- [ ] Test JSON error shape
- [ ] Test account resolution precedence
- [ ] Test config precedence
- [x] Test keystore roundtrip
- [ ] Test confirmation / `--yes` behavior
- [ ] Test `--dry-run` behavior
- [x] Test EVM address derivation
- [x] Test wrong password rejection

## Phase 0 done when
- [x] Heat foundation behavior is stable enough that protocol work can start without redefining core contracts

---

# Phase 1 - Hyperliquid

## 1.0 Protocol intake
- [x] Do protocol interview before implementation
- [x] Confirm official SDK dependency choice
- [x] Confirm what Hyperliquid CLI logic to reuse directly
- [x] Confirm what stays Heat-native
- [x] Confirm first-pass command scope

## 1.1 Dependency and reuse setup
- [x] Add `hypersdk`
- [x] Pin reference version / commit in `UPSTREAM.md`
- [x] Identify specific upstream files / logic to adapt
- [x] Avoid wholesale blind CLI copy

## 1.2 Read/query commands
- [x] `heat hl price`
- [x] `heat hl perps`
- [x] `heat hl spot`
- [x] `heat hl balance`
- [x] `heat hl positions`
- [x] `heat hl orders`

## 1.3 Write commands
- [x] `heat hl buy`
- [x] `heat hl sell`
- [x] `heat hl cancel`
- [x] `heat hl leverage` (show + set, set uses raw exchange signing)
- [x] `heat hl send`

## 1.4 Streams
- [x] `heat hl stream trades`
- [x] Ensure stream output is NDJSON
- [x] Ensure stream diagnostics go to stderr

## 1.5 Hyperliquid-specific validation
- [x] asset / market resolution (fuzzy matching with suggestions)
- [x] size / price validation
- [x] leverage validation (asset resolution, cross/isolated, dry-run)
- [x] incompatible flag handling

## 1.6 Safety
- [x] Dry-run support where practical
- [x] Confirmation behavior on dangerous writes
- [x] Clear preview before execution where possible
- [x] Network validation (reject unknown values, prevent silent mainnet fallback)
- [x] Order rejection detection (check OrderResponseStatus::Error)

## 1.7 Output polishing
- [x] Good pretty output for common HL queries
- [x] Clean raw JSON for machine mode
- [x] Sensible quiet-mode outputs where useful

## 1.8 Tests
- [x] Asset resolution tests (6 tests in heat-hl)
- [ ] Write command dry-run tests
- [ ] Error handling tests
- [ ] Stream format tests if feasible

## Phase 1 done when
- [ ] Hyperliquid is practically usable for human and agent workflows inside Heat

### Remaining for Phase 1 completion
- [x] Leverage show (reads from clearinghouse state)
- [x] Leverage set (raw exchange signing, bypasses SDK gap)
- [x] Stream trades (live-tested against mainnet)
- [ ] More integration tests
- [ ] Manual testing of all write commands on testnet

---

# Phase 2 - Polymarket

## 2.0 Protocol intake
- [x] Do protocol interview before implementation
- [x] Confirm official client crate (`polymarket-client-sdk 0.4.3`)
- [x] Confirm what `polymarket-cli` logic to reuse directly
- [x] Confirm first-pass command scope

## 2.1 Dependency and reuse setup
- [x] Add `polymarket-client-sdk`
- [x] Pin reference version / commit in `UPSTREAM.md`
- [x] Identify auth / approval / market logic to adapt

## 2.2 Market browsing
- [x] `heat pm gamma markets` (list/get/search via Gamma API)
- [x] `heat pm clob markets` / `heat pm clob simplified-markets` (CLOB market data)
- [x] `heat pm data` subcommands (positions, trades, activity, leaderboard, etc.)

## 2.3 Trading / CLOB
- [x] `heat pm price` (helper → clob book)
- [x] `heat pm buy` (helper → clob limit-order side=buy)
- [x] `heat pm sell` (helper → clob limit-order side=sell)
- [x] `heat pm cancel` (helper → clob cancel-order)
- [x] `heat pm orders` (helper → clob open-orders)
- [x] `heat pm trades` (helper → clob trades)
- [x] `heat pm positions` (helper → data positions)
- [x] `heat pm balance` (helper → clob balance)

## 2.4 Approvals / protocol-specific setup
- [x] `heat pm approve check` (real on-chain ERC20 allowance + ERC1155 approval for all 3 targets)
- [x] `heat pm approve set` (on-chain max approval for all 3 exchange contracts)
- [x] Support protocol-specific signature type through account/config (`protocols.polymarket.signature_type`)

## 2.5 Polymarket-specific validation
- [x] token id validation (decimal string parsing to U256)
- [x] side validation (buy/sell string matching)
- [x] signature type compatibility (proxy/eoa/gnosis-safe with Result-based parsing)
- [x] auth/account requirement validation

## 2.6 Safety
- [x] Dry-run support where practical (approve set, clob limit-order, clob cancel)
- [x] Confirmation behavior on dangerous writes (approve set uses confirm_dangerous)

## 2.7 Output polishing
- [x] Good pretty output for market browsing
- [x] Clean JSON output for agents (all Heat-owned DTOs, no Debug strings)
- [x] PM stream output uses Heat-owned DTOs and NDJSON-friendly machine output

## 2.8 Streaming
- [x] `heat pm stream orderbook`
- [x] `heat pm stream prices`
- [x] `heat pm stream midpoints`
- [x] `heat pm stream orders`
- [x] `heat pm stream trades`
- [x] Ensure stream diagnostics go to stderr
- [x] Reject unsupported quiet mode for PM streams

## 2.9 Tests
- [x] Auth / account resolution tests (11 tests: signature type parsing, wallet derivation)
- [x] Bridge output tests (4 tests: status labels, safety invariant)
- [x] Data DTO serialization tests (10 tests: all major DTOs)
- [x] CLOB output tests (9 tests: side/status labels, DTO shapes)
- [x] Stream parsing / mapping / serialization tests
- [x] Crate-level integration tests (2 tests)
- [ ] Dry-run tests
- [ ] More integration tests

## Phase 2 done when
- [x] Hyperliquid and Polymarket both feel like the same product while respecting protocol differences

### Remaining hardening after Phase 2 acceptance
- [ ] Manual testing of all write commands on mainnet
- [ ] More integration tests
- [ ] ApiKeys command blocked on SDK (ApiKeysResponse fields private)

---

# Shared EVM substrate (pre-work for Phase 3 / 4)

## EVM base extraction
- [x] Decide shared crate/module location (`heat-evm` vs `heat-core` module)
- [x] Define canonical chain names (`ethereum`, `polygon`, `arbitrum`, `optimism`, `base`)
- [x] Define chain ID mapping and validation helpers
- [x] Define shared RPC resolution precedence
- [x] Extract reusable EVM signer resolution from Heat accounts
- [x] Extract readonly + wallet provider helpers
- [x] Extract shared EVM address / amount parsing helpers
- [ ] Confirm how existing HL / PM code should migrate incrementally

---

# Phase 3 - LI.FI

## 3.0 Protocol intake
- [x] Do protocol interview before implementation
- [x] Confirm official SDK/API usage path
- [x] Confirm minimal first scope
- [x] Confirm what is intentionally deferred
- [x] Write LI.FI implementation brief

## 3.1 Minimal command scope
- [x] `heat lifi quote`
- [x] `heat lifi routes`
- [x] `heat lifi bridge` (full execution: route selection, ERC-20 approval, tx send)
- [x] `heat lifi status`
- [x] `heat lifi chains`
- [x] `heat lifi tokens`
- [x] `heat lifi tools`

## 3.2 Validation / safety
- [x] validate chain names (Heat canonical names via EvmChain + numeric fallback)
- [x] validate token inputs (symbol/address resolution via LI.FI /tokens)
- [x] preview route clearly in dry-run mode (DryRunPreview with route details)
- [x] clearly surface approval requirements when known (approval_address from estimates)
- [x] TTY confirmation for bridge execution (confirm_dangerous)
- [x] --yes override for non-interactive use

## 3.3 Output
- [x] readable route summary in pretty mode
- [x] raw route payload in JSON mode (BridgeResultDto, StepResultDto)
- [x] execution status output

## 3.4 Execution classification
- [x] ExecutionFamily classification (EVM, Solana, Unsupported)
- [x] Static chain ID fallback + chain type metadata override
- [x] Route-level and step-level classification
- [x] EVM routes marked as supported; non-EVM rejected

## 3.5 Adversarial review fixes
- [x] Inject action.fromAddress into stepTransaction requests
- [x] Tighten execution classification to Heat-supported chains only
- [x] Gas limit parsing handles hex + decimal (parse_value_flexible)
- [x] Read vs bridge scope documented in help text
- [x] Native token / ERC-20 approval tests added
- [x] HTTP tests for stepTransaction body shape

## Phase 3 done when
- [x] Heat can quote, inspect, execute, and check route status safely enough for first bridge workflows

---

# Phase 4 - Lending / Strategy Protocols

## 4.0 Protocol intake
- [ ] Do protocol interview before each protocol
- [ ] Confirm SDK/client source
- [ ] Confirm signer/account needs
- [ ] Confirm first command scope

## Candidate first command sets

### Aave
- [x] Fetch and mirror official Aave docs locally
- [x] Capture official address references for Ethereum / Arbitrum / Base
- [x] Write Aave phase-1 implementation brief
- [x] Add `heat-aave` crate
- [x] `markets`
- [x] `positions`
- [x] `health`
- [x] `supply`
- [x] `withdraw`
- [x] Adversarial review pass (zero-amount validation, dead field cleanup)
- [ ] borrow
- [ ] repay

### Kamino / Pacific class
- [ ] deposit / withdraw
- [ ] borrow / repay if relevant
- [ ] positions
- [ ] reserve / market info

## Phase 4 done when
- [ ] Heat proves it can extend beyond trading and bridging into lending-style workflows

---

# Ongoing project hygiene

## Docs and planning
- [x] Keep `ROADMAP.md` updated as phases move
- [x] Keep `CHECKLIST.md` updated with actual progress
- [x] Keep `UPSTREAM.md` updated when dependencies or references change
- [ ] Record protocol interview decisions before each new integration

## Release quality
- [x] Do not add protocols before the core contract is stable enough
- [x] Do not add nice-to-have features ahead of current phase goals
- [x] Prefer finishing one protocol well over starting three halfway

---

# Explicitly deferred
- [ ] hardware wallets
- [ ] multisig support
- [ ] plugin SDK
- [ ] TUI
- [ ] portfolio analytics
- [ ] strategy automation
- [ ] background daemons

These should remain deferred unless priorities change explicitly.
