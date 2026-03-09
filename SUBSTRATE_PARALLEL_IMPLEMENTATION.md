# Heat CLI - Parallel Substrate & Protocol Implementation Brief

Internal working doc for the next implementation wave.

This document is for developers working in parallel on:
- shared EVM substrate
- shared Solana substrate
- LI.FI
- Aave
- Pacifica

Read this before starting work.

---

## 0. Why this document exists

Heat is about to expand beyond the current two accepted protocol integrations:
- Hyperliquid
- Polymarket

The next wave is more dangerous architecturally because it spans **multiple chain families**:
- **EVM**: LI.FI execution first pass, Aave, future PM shared helpers
- **Solana**: Pacifica, future Solana protocols

If we scope these in isolation, we will likely make one of these mistakes:
- leak EVM assumptions into `heat-core`
- build `heat-evm` like a fake universal chain layer
- force Pacifica to invent its own signer/RPC/account model separately
- make LI.FI look universal in output but only accidentally executable on one family
- duplicate account, amount, RPC, and signer logic inside protocol crates

This document freezes the intended architecture and parallel work plan.

---

## 1. Product and architecture goals

Heat remains:
- protocol-first at the top level
- helper-first where practical
- shared-core underneath
- safety-aware for real money operations
- equally optimized for humans and agents

Top-level UX remains:

```bash
heat <protocol> <command>
```

Examples of the next wave:

```bash
heat lifi quote --from-chain ethereum --to-chain polygon --from-token ETH --to-token USDC --amount 1
heat aave markets --chain ethereum
heat aave supply USDC 1000 --chain base --account main
heat pacifica positions --account sol-main
```

What is shared across all protocols:
- output contract
- error contract
- config precedence
- account-first UX
- safety behavior
- dry-run behavior
- stdout/stderr contract

What is **not** fake-unified:
- EVM and Solana signer mechanics
- address types
- token standards
- RPC behavior
- contract/program invocation
- transaction building

---

## 2. Hard architecture rules

### 2.1 `heat-core` stays family-neutral

`heat-core` is the universal product core.

It may own:
- output modes
- errors
- config precedence
- account metadata and account selection
- runtime context
- safety / confirmation / dry-run policy
- generic validation helpers
- filesystem helpers

It must **not** own:
- Alloy EVM primitives
- Solana SDK types
- chain IDs
- `Address`
- `Pubkey`
- ERC20 helpers
- SPL helpers
- gas / nonce / blockhash / ATA logic
- contract / program call builders

### 2.2 Family plumbing lives in family crates

Create and maintain these crates:
- `crates/heat-evm`
- `crates/heat-solana`

Use them as family-specific helper substrates.

Do **not** create a giant fake universal crate like:
- `heat-chain`
- `heat-web3`
- `heat-blockchain`

Those abstractions will be wrong too early.

### 2.3 Protocol crates own protocol semantics

Protocol crates own:
- command trees
- protocol DTOs
- protocol validation
- protocol-specific auth details
- protocol-specific output shaping
- protocol-specific safety classification

Substrates own family mechanics only.

### 2.4 Heat owns output contracts

No protocol should expose upstream SDK output as Heat’s stable public contract by default.

Pattern:
1. call SDK/client/contract/program
2. map to Heat-owned DTOs
3. render through `heat-core` output mode layer

---

## 3. Repo target shape

### Existing
- `crates/heat-core`
- `crates/heat-cli`
- `crates/heat-hl`
- `crates/heat-pm`

### New
- `crates/heat-evm`
- `crates/heat-solana`
- `crates/heat-lifi`
- `crates/heat-aave`
- `crates/heat-pacifica`

### Dependency direction

Allowed:
- `heat-cli` -> `heat-core`, protocol crates
- protocol crates -> `heat-core`, family substrate crate(s)
- `heat-evm` -> `heat-core`
- `heat-solana` -> `heat-core`

Not allowed:
- `heat-core` -> `heat-evm`
- `heat-core` -> `heat-solana`
- `heat-evm` -> `heat-solana`
- `heat-solana` -> `heat-evm`
- protocol crates reaching into each other for substrate logic

---

## 4. Account model evolution

### 4.1 Current problem

Heat currently has an account model shaped mainly around EVM-backed accounts.
That was acceptable for Hyperliquid and Polymarket.
It is not sufficient for Pacifica.

### 4.2 Target model

Move toward capability-based account kinds in `heat-core`.

Representative direction:

```rust
pub enum AccountKind {
    EvmLocal,
    SolanaLocal,
}
```

Possible later expansion:

```rust
pub enum AccountKind {
    EvmLocal,
    SolanaLocal,
    EvmKms,
    SolanaLedger,
}
```

### 4.3 User-facing model stays account-first

Users should still think in terms of:
- `heat accounts ...`
- `--account <NAME>`

Protocol/substrate resolution decides if the selected account is compatible.

Examples:
- `heat aave positions --account main`
  - requires EVM-capable account
- `heat pacifica positions --account main`
  - requires Solana-capable account

### 4.4 Account metadata caching

Allow family-appropriate cached public identity in account metadata where useful:
- EVM account: cached `0x...` address
- Solana account: cached base58 pubkey

This is useful for read-only UX without decrypting secrets.

### 4.5 Compatibility errors

Add clear compatibility errors in core/substrate resolution:
- account exists but wrong family
- account missing required key material
- account kind unsupported by protocol

Do not let each protocol invent its own mismatch error wording.

---

## 5. Shared runtime principles

### 5.1 `Ctx` stays neutral

`Ctx` may carry:
- output mode
- selected account name
- raw requested network string
- dry-run
- yes
- config
- tty metadata

`Ctx` should **not** carry typed family fields like:
- `chain_id: u64`
- `evm_address: Address`
- `solana_pubkey: Pubkey`

Those belong in substrate helpers.

### 5.2 Network handling

Do not over-unify EVM chains and Solana clusters into one premature core enum.

Use this rule instead:
- core stores the user-requested network string or protocol-local config value
- EVM protocols interpret it as an EVM chain
- Solana protocols interpret it as a Solana cluster

This keeps the product layer simple without forcing a fake common type.

### 5.3 Dry-run contract

Product contract:
- `--dry-run` means do as much validation/preparation/simulation as practical
- do not commit final state-changing operation

Implementation differs by family:
- EVM: build tx, estimate/simulate where practical, do not send final tx
- Solana: build instruction/transaction, simulate where practical, do not submit final tx

The user should experience one Heat contract, not two incompatible ones.

---

## 6. EVM substrate design (`crates/heat-evm`)

### 6.1 Purpose

Provide thin Heat-owned EVM helpers over Alloy for:
- LI.FI
- Aave
- future PM helper migration
- future EVM protocols

### 6.2 Reuse policy

Use Alloy directly for:
- primitives
- providers
- signers
- contracts
- transactions

Use `alloy-chains` where useful for:
- chain IDs
- named chain parsing / aliasing
- metadata helpers

Heat still owns:
- canonical CLI naming
- RPC resolution policy
- account/signer resolution from Heat accounts
- amount conversion policy
- error wording / output contract

### 6.3 Proposed module tree

```text
crates/heat-evm/
  Cargo.toml
  src/
    lib.rs
    chains.rs
    rpc.rs
    signer.rs
    erc20.rs
    parse.rs
    amount.rs
```

### 6.4 `chains.rs`

Responsibilities:
- define Heat-supported canonical EVM chains
- parse CLI chain names and accepted aliases
- expose canonical output names
- expose chain IDs
- optionally map to `alloy_chains::NamedChain`
- expose native token symbol helpers

Initial supported EVM chains:
- `ethereum`
- `polygon`
- `arbitrum`
- `optimism`
- `base`

Possible later additions only when truly required:
- HyperEVM
- Avalanche
- BNB Chain

Representative API:

```rust
pub enum EvmChain {
    Ethereum,
    Polygon,
    Arbitrum,
    Optimism,
    Base,
}

impl EvmChain {
    pub fn canonical_name(&self) -> &'static str;
    pub fn chain_id(&self) -> u64;
    pub fn native_symbol(&self) -> &'static str;
    pub fn alloy_named(&self) -> Option<alloy_chains::NamedChain>;
}
```

Rule:
- Heat accepts upstream/common aliases if helpful
- Heat outputs canonical names consistently

### 6.5 `rpc.rs`

Responsibilities:
- resolve RPC URL for a chain
- implement Heat precedence rules
- support explicit override from protocol commands
- return clear errors on missing configuration

Suggested precedence:
1. explicit command flag
2. protocol config override
3. generic Heat env var
4. generic Heat config
5. optional built-in default if policy allows

Suggested environment variables:
- `HEAT_RPC_ETHEREUM`
- `HEAT_RPC_POLYGON`
- `HEAT_RPC_ARBITRUM`
- `HEAT_RPC_OPTIMISM`
- `HEAT_RPC_BASE`

Representative API:

```rust
pub fn resolve_rpc_url(
    ctx: &Ctx,
    chain: EvmChain,
    explicit: Option<&str>,
) -> Result<String, HeatError>;
```

### 6.6 `signer.rs`

Responsibilities:
- verify selected account is EVM-capable
- decrypt/load local EVM signer
- derive and return EOA address
- build read-only provider
- build signer-backed provider

Representative API:

```rust
pub fn resolve_eoa_address(ctx: &Ctx) -> Result<Address, HeatError>;
pub fn private_key_signer(ctx: &Ctx) -> Result<PrivateKeySigner, HeatError>;
pub async fn read_provider(ctx: &Ctx, chain: EvmChain) -> Result<impl Provider, HeatError>;
pub async fn wallet_provider(ctx: &Ctx, chain: EvmChain) -> Result<impl Provider, HeatError>;
```

### 6.7 `erc20.rs`

Responsibilities:
- minimal reusable ERC20 helpers only

Required helpers:
- `symbol`
- `decimals`
- `balance_of`
- `allowance`
- `approve`

Do **not** put Aave or LI.FI semantics here.

### 6.8 `parse.rs`

Responsibilities:
- address parsing
- chain parsing helper if needed outside `chains.rs`
- shared U256 parsing helpers
- CLI validation support

### 6.9 `amount.rs`

Recommended from day one.

Responsibilities:
- decimal string -> base units
- base units -> decimal string
- exact, string-safe conversions

Representative API:

```rust
pub fn parse_units(input: &str, decimals: u8) -> Result<U256, HeatError>;
pub fn format_units(value: U256, decimals: u8) -> String;
```

This is important for:
- Aave amounts
- LI.FI quotes/execution
- future ERC20 interactions

### 6.10 Acceptance criteria for `heat-evm`

`heat-evm` is accepted when:
- chain parsing is centralized
- RPC resolution is centralized
- account -> signer/provider resolution is centralized
- ERC20 helper exists
- amount conversion is safe and tested
- Aave and LI.FI can use it without protocol-local EVM boilerplate duplication

---

## 7. Solana substrate design (`crates/heat-solana`)

### 7.1 Purpose

Provide thin Heat-owned Solana helpers for:
- Pacifica
- later Jupiter/Kamino/etc
- later LI.FI Solana execution if desired

### 7.2 Why this is needed now even if implementation is partial

Pacifica is Solana-native.
If we only implement EVM substrate first without defining the Solana boundary, we risk:
- leaking EVM assumptions into core
- forcing Pacifica to create ad hoc Solana plumbing
- making LI.FI look architecturally broader than it is

So the Solana substrate should be scoped now even if implemented in phases.

### 7.3 Proposed module tree

```text
crates/heat-solana/
  Cargo.toml
  src/
    lib.rs
    cluster.rs
    rpc.rs
    signer.rs
    parse.rs
    spl.rs
    tx.rs
    amount.rs
```

### 7.4 `cluster.rs`

Responsibilities:
- define supported Solana clusters
- parse canonical cluster names
- expose canonical output naming

Initial supported clusters:
- `mainnet`
- `devnet`

Representative API:

```rust
pub enum SolanaCluster {
    Mainnet,
    Devnet,
}

impl SolanaCluster {
    pub fn canonical_name(&self) -> &'static str;
}
```

Use Solana-native terminology internally: `cluster`.

### 7.5 `rpc.rs`

Responsibilities:
- resolve cluster RPC URL
- support explicit override
- support Heat env/config resolution
- return clear errors

Suggested environment variables:
- `HEAT_RPC_SOLANA`
- optional later `HEAT_PACIFICA_SOLANA_RPC`

Representative API:

```rust
pub fn resolve_rpc_url(
    ctx: &Ctx,
    cluster: SolanaCluster,
    explicit: Option<&str>,
) -> Result<String, HeatError>;
```

### 7.6 `signer.rs`

Responsibilities:
- verify account is Solana-capable
- load/decrypt Solana keypair
- resolve pubkey
- return signer/keypair for tx signing

Representative API:

```rust
pub fn resolve_pubkey(ctx: &Ctx) -> Result<Pubkey, HeatError>;
pub fn resolve_keypair(ctx: &Ctx) -> Result<Keypair, HeatError>;
```

Important:
- protocol crates should not each implement their own keypair parsing/decryption path

### 7.7 `parse.rs`

Responsibilities:
- parse pubkeys
- validate base58 inputs where relevant
- shared CLI validation helpers

### 7.8 `spl.rs`

Responsibilities:
- associated token account derivation
- SPL token balance lookup
- token transfer helpers
- ATA helper(s) if needed

This is required for Pacifica-adjacent wallet/deposit/transfer work.

### 7.9 `tx.rs`

Responsibilities:
- blockhash fetch
- tx simulation
- tx send
- confirmation helpers
- dry-run support hooks

No giant universal abstraction. Keep it practical.

### 7.10 `amount.rs`

Recommended from day one.

Responsibilities:
- decimal string -> lamports/token base units
- base units -> display string
- exact conversion

### 7.11 Acceptance criteria for `heat-solana`

`heat-solana` is accepted as the initial substrate when:
- cluster parsing is centralized
- RPC resolution is centralized
- Solana signer shape is centralized
- Pacifica can use it without inventing separate account/RPC plumbing
- no Solana assumptions leak into `heat-core`

---

## 8. LI.FI design (`crates/heat-lifi`)

### 8.1 Integration choice

Use direct LI.FI REST integration in Rust.
Do **not** use the TypeScript SDK as the primary runtime path.
Use the local `reference/lifi-sdk/` tree as a behavioral/reference source only.

### 8.2 Why LI.FI matters architecturally

LI.FI can span chain families.
Heat cannot pretend all family execution is implemented at once.
So LI.FI must be designed with a split between:
- route visibility / read path
- execution capability

### 8.3 Proposed module tree

```text
crates/heat-lifi/
  Cargo.toml
  src/
    lib.rs
    cmd.rs
    client.rs
    dto.rs
    map.rs
    exec.rs
    tests.rs
```

### 8.4 First-pass scope

Read path first:
- `chains`
- `tokens`
- `tools`
- `quote`
- `routes`
- `status`

Execution first pass:
- **EVM-only execution**
- Solana execution explicitly unsupported initially

### 8.5 Execution capability model

Add internal execution capability classification.
Representative direction:

```rust
pub enum ExecutionFamily {
    Evm,
    Solana,
    Unsupported,
}
```

Or a richer route support classifier.

Heat should be able to say:
- this route is visible
- this route is not yet executable by current Heat runtime
- reason: requires unsupported execution family/step

This is especially important for agents.

### 8.6 Output contract

LI.FI outputs must be Heat-shaped.
Do not expose raw upstream REST blobs as the public contract by default.

### 8.7 Acceptance criteria for `heat-lifi` phase 1

Accepted when:
- chains/tokens/quote/routes/status work
- output is Heat-owned and stable
- execution support is clearly classified
- unsupported Solana execution is rejected honestly, not silently

---

## 9. Aave design (`crates/heat-aave`)

### 9.1 Integration choice

Use contract-first Alloy integration.
Do not wait for a nonexistent official Rust SDK.
Use:
- Aave docs
- Aave V3 Pool / PoolAddressesProvider
- `aave-address-book` or official address sources

### 9.2 Proposed module tree

```text
crates/heat-aave/
  Cargo.toml
  src/
    lib.rs
    cmd.rs
    addresses.rs
    contracts.rs
    read.rs
    write.rs
    dto.rs
    tests.rs
```

### 9.3 First-pass scope

Read:
- `markets`
- `positions`
- `health`

Write:
- `supply`
- `withdraw`

Deferred:
- `borrow`
- `repay`
- collateral toggles
- e-mode
- liquidation helpers

### 9.4 Substrate usage

`heat-aave` should consume:
- `heat-evm::chains`
- `heat-evm::rpc`
- `heat-evm::signer`
- `heat-evm::erc20`
- `heat-evm::amount`

It should not build its own signer/provider stack.

### 9.5 Acceptance criteria for `heat-aave` phase 1

Accepted when:
- read path works with Heat-owned outputs
- Aave addresses are sourced from official references
- write path uses shared EVM helpers
- protocol-specific behavior is kept in `heat-aave`, not substrate code

---

## 10. Pacifica design (`crates/heat-pacifica`)

### 10.1 What we learned from the reference

`reference/regatta/` shows Pacifica is clearly Solana-native.
The reference tooling uses:
- Solana-style keypairs
- Solana RPC
- Solana deposit and transfer helpers
- Pacifica REST + WS APIs

So Pacifica should be built on `heat-solana`, not as a one-off protocol with bespoke signing and RPC logic.

### 10.2 Proposed module tree

```text
crates/heat-pacifica/
  Cargo.toml
  src/
    lib.rs
    cmd.rs
    client.rs
    ws.rs
    auth.rs
    deposit.rs
    transfer.rs
    dto.rs
    tests.rs
```

### 10.3 First-pass scope

Read:
- account
- positions
- orders
- prices
- orderbook/book
- candles if practical

Write:
- buy
- sell
- cancel
- leverage/margin if confirmed in protocol interview scope
- deposit
- withdraw
- SOL/USDC transfers if retained in Heat scope

### 10.4 Substrate usage

`heat-pacifica` should consume:
- `heat-solana::cluster`
- `heat-solana::rpc`
- `heat-solana::signer`
- `heat-solana::spl`
- `heat-solana::tx`

### 10.5 Acceptance criteria for `heat-pacifica` phase 1

Accepted when:
- signer and RPC logic are not duplicated protocol-locally
- account/pubkey resolution is shared with substrate
- protocol outputs are Heat-owned
- Pacifica-specific program/REST/WS logic stays protocol-local

---

## 11. Research requirements by team

Every developer must do source-backed research before and during implementation.
Do not rely on memory or vague assumptions.

### 11.1 Required local reference trees

Inspect these directly:
- `reference/lifi-sdk/`
- `reference/regatta/`
- `reference/viem/` (reference only, not implementation basis)
- `reference/polymarket-cli/`
- `reference/rs-clob-client/`
- `reference/hlz/`
- `reference/hypersdk/`

Also inspect installed crates where relevant, especially:
- Alloy crates in cargo registry
- `alloy-chains`
- any Solana crates chosen for the substrate

### 11.2 Dev 1 research checklist - core/account model

Read and inspect:
- `PLAN.md`
- `ROADMAP.md`
- `CHECKLIST.md`
- `UPSTREAM.md`
- `RESEARCH.md`
- `crates/heat-core/src/accounts.rs`
- `crates/heat-core/src/ctx.rs`
- `crates/heat-core/src/config.rs`
- `crates/heat-core/src/output.rs`
- `crates/heat-core/src/safety.rs`
- `crates/heat-core/src/error.rs`
- `crates/heat-core/src/keystore.rs`
- `crates/heat-cli/src/cmd_accounts.rs`

Questions to answer before coding:
- How do we represent account kind without breaking existing EVM users?
- Where should cached public identity live?
- How do we preserve current account-first UX while allowing family compatibility checks?
- What changes are needed to config and keystore persistence?

### 11.3 Dev 2 research checklist - EVM substrate + Aave

Read and inspect:
- installed Alloy crates, especially `alloy-chains`, `alloy-provider`, `alloy-signer-local`, `alloy-contract`
- `reference/viem/src/chains/index.ts`
- `reference/viem/src/chains/definitions/mainnet.ts`
- `reference/lifi-sdk/packages/sdk/src/actions/getChains.ts`
- `reference/lifi-sdk/packages/sdk/src/actions/getTokens.ts`
- `reference/lifi-sdk/packages/sdk/src/actions/getQuote.ts`
- `reference/lifi-sdk/packages/sdk/src/actions/getRoutes.ts`
- `reference/lifi-sdk/packages/sdk/src/actions/getStatus.ts`
- `reference/lifi-sdk/packages/sdk/src/actions/getStepTransaction.ts`
- Aave official docs and address sources already captured in `RESEARCH.md`

Questions to answer before coding:
- Exactly how much chain metadata should Heat own vs inherit from Alloy?
- What is the minimal EVM chain set for first pass?
- What exact RPC precedence should EVM protocols follow?
- What exact amount conversion helpers are needed to avoid `f64` and duplication?
- What Aave contract bindings and address sources are sufficient for read path first?

### 11.4 Dev 3 research checklist - Solana substrate + Pacifica

Read and inspect:
- `reference/regatta/regatta-cli/README.md`
- `reference/regatta/regatta-cli/src/sdk/config.zig`
- `reference/regatta/regatta-cli/src/sdk/signing.zig`
- `reference/regatta/regatta-cli/src/sdk/solana.zig`
- `reference/regatta/regatta-cli/src/sdk/client.zig`
- `reference/regatta/regatta-cli/src/sdk/ws.zig`
- any chosen Rust Solana SDK/client crates before coding

Questions to answer before coding:
- What Rust Solana dependency path is mature and practical right now?
- What account format should Heat store for Solana local accounts?
- How should Solana RPC precedence map into Heat config/env/flags?
- What belongs in substrate (`spl`, `tx`, signer) vs what belongs only in Pacifica protocol code?

### 11.5 Dev 4 research checklist - LI.FI

Read and inspect:
- `reference/lifi-sdk/README.md`
- `reference/lifi-sdk/examples/node/examples/bridge.ts`
- `reference/lifi-sdk/examples/node/examples/swap.ts`
- `reference/lifi-sdk/packages/sdk/src/index.ts`
- `reference/lifi-sdk/packages/sdk/src/core/execution.ts`
- all `get*` action files listed above
- LI.FI docs/llms entry already noted in `RESEARCH.md`

Questions to answer before coding:
- Which endpoints are sufficient for Heat phase 1?
- How should Heat classify route execution capability?
- What route fields matter for human pretty output vs machine output?
- Which execution fields should be exposed later without locking a bad contract now?

---

## 12. Parallel workstreams

This work is intended to be split across developers.
Each workstream should produce reviewable PRs with tests.

### Workstream A - Core boundary freeze

Owner: Dev 1

Goal:
- make `heat-core` family-neutral while supporting multi-family accounts

Tasks:
1. Add account kind/capability model
2. Ensure `Ctx` remains neutral
3. Add shared compatibility-check helpers
4. Add tests for account mismatch and resolution behavior
5. Update account persistence model if required

Deliverables:
- updated core account model
- no EVM/Solana leakage into core

### Workstream B - EVM substrate

Owner: Dev 2

Goal:
- implement `heat-evm` thinly but completely enough for LI.FI and Aave

Tasks:
1. Create crate and wire workspace
2. Implement `chains.rs`
3. Implement `rpc.rs`
4. Implement `signer.rs`
5. Implement `erc20.rs`
6. Implement `parse.rs`
7. Implement `amount.rs`
8. Add tests

Deliverables:
- usable EVM substrate for protocol crates

### Workstream C - Solana substrate

Owner: Dev 3

Goal:
- define and implement the initial `heat-solana` shape so Pacifica can build cleanly

Tasks:
1. Create crate and wire workspace
2. Implement `cluster.rs`
3. Implement `rpc.rs`
4. Implement `parse.rs`
5. Implement initial `signer.rs`
6. Add `amount.rs`
7. Define `spl.rs` / `tx.rs` boundary
8. Add tests

Deliverables:
- stable Solana substrate boundary
- enough implementation that Pacifica does not need bespoke signer/RPC logic

### Workstream D - LI.FI

Owner: Dev 4 if available, otherwise after EVM substrate lands

Goal:
- ship Heat-native LI.FI read path

Tasks:
1. Create crate and wire workspace
2. Implement client and DTOs
3. Add commands for chains/tokens/tools/quote/routes/status
4. Add execution support classification
5. Gate actual execution to supported family capability
6. Add tests

Deliverables:
- LI.FI read path
- route visibility broader than current execution support

### Workstream E - Aave

Owner: can start after basic `heat-evm` scaffolding is in place

Goal:
- build Aave on top of `heat-evm`

Tasks:
1. Create crate and wire workspace
2. Add addresses and contract bindings
3. Implement `markets`
4. Implement `positions`
5. Implement `health`
6. Later implement `supply` and `withdraw`
7. Add tests

Deliverables:
- Aave read path first

### Workstream F - Pacifica

Owner: can start after `heat-solana` boundary is in place

Goal:
- build Pacifica on top of `heat-solana`

Tasks:
1. Create crate and wire workspace
2. Implement protocol client/auth/DTOs
3. Implement read commands first
4. Implement write path based on confirmed protocol scope
5. Add tests

Deliverables:
- Pacifica protocol crate with shared Solana substrate usage

---

## 13. PR sequencing guidance

Recommended merge order:

1. Core account/capability model changes
2. `heat-evm` scaffold + implementation
3. `heat-solana` scaffold + initial implementation
4. `heat-lifi` read path
5. `heat-aave` read path
6. `heat-pacifica` read path
7. write-path expansions

Important:
- `heat-solana` does not need full Pacifica implementation before merge
- but its boundary should land early enough to constrain architecture

---

## 14. Testing expectations

Every workstream must add tests.

### Core tests
- account kind mismatch
- account resolution precedence
- config/env/flag precedence for any new account/network fields

### EVM substrate tests
- chain parsing
- chain ID mapping
- RPC precedence
- signer/address resolution
- amount conversion exactness

### Solana substrate tests
- cluster parsing
- RPC precedence
- pubkey parsing
- signer/account compatibility checks
- amount conversion exactness

### Protocol crate tests
- DTO serialization
- unsupported capability rejections
- route/command validation
- dry-run behavior where applicable

---

## 15. Explicit non-goals for this wave

Do **not** build any of these right now:
- giant universal blockchain abstraction
- hardware wallet support
- multisig support
- universal route execution engine across all families
- giant token registry
- multicall/permit frameworks
- plugin system
- TUI
- portfolio engine
- strategy automation

Keep the substrate practical and thin.

---

## 16. Existing code that should influence design

Study these current Heat files before changing architecture:
- `crates/heat-core/src/accounts.rs`
- `crates/heat-core/src/config.rs`
- `crates/heat-core/src/ctx.rs`
- `crates/heat-core/src/error.rs`
- `crates/heat-core/src/keystore.rs`
- `crates/heat-core/src/output.rs`
- `crates/heat-core/src/safety.rs`
- `crates/heat-cli/src/cmd_accounts.rs`
- `crates/heat-pm/src/auth.rs`
- `crates/heat-pm/src/cmd/approve.rs`
- `crates/heat-hl/src/signer.rs`

These already contain patterns that should be centralized or preserved.

---

## 17. Existing external references that must influence design

### For EVM
- local Alloy crates in cargo registry
- `reference/viem/` for chain metadata/reference expectations only
- `reference/lifi-sdk/` for endpoint/route semantics
- Aave official docs and address sources

### For Solana / Pacifica
- `reference/regatta/regatta-cli/README.md`
- `reference/regatta/regatta-cli/src/sdk/*.zig`

### For Heat behavior consistency
- `reference/hlz/`
- `reference/polymarket-cli/`
- `reference/rs-clob-client/`

---

## 18. Definition of success for this wave

This wave is successful when:
- `heat-core` remains family-neutral
- both `heat-evm` and `heat-solana` exist with clear boundaries
- LI.FI can expose broad route information without lying about execution support
- Aave uses `heat-evm` instead of bespoke EVM plumbing
- Pacifica uses `heat-solana` instead of bespoke Solana plumbing
- output contracts remain Heat-owned
- developers can add future EVM or Solana protocols without repeating signer/RPC/account boilerplate

---

## 19. Immediate next actions

1. Read this document completely
2. Read `PLAN.md`, `ROADMAP.md`, `CHECKLIST.md`, `UPSTREAM.md`, `RESEARCH.md`
3. Inspect the reference trees listed above
4. Create issue/PR breakdown matching the workstreams
5. Start with boundary-safe scaffolding, not protocol-specific hacks

If unsure:
1. choose the simpler design
2. keep `heat-core` neutral
3. prefer thin family substrates over fake universal abstractions
4. reuse mature upstream libraries for low-level mechanics
5. keep Heat’s product contracts owned by Heat
