# AAVE_IMPLEMENTATION_BRIEF.md

This brief is for the developer implementing Heat's first Aave integration.

Read this before writing code.

---

## Do we need more than one developer?

### Recommendation
**No. One strong developer is enough for the current Aave phase-1 scope.**

Why:
- scope is intentionally small
- architecture decisions are still fresh and should stay coherent
- Aave phase 1 is the first real proving ground for `heat-evm`
- splitting early would likely create read/write/output drift

### When two developers would make sense
Only split if we need faster delivery and have a clear reviewer/owner model.

If we do split, use exactly this boundary:
- **Dev A:** read path (`markets`, `positions`, `health`)
- **Dev B:** write path (`supply`, `withdraw`) plus approval flow and safety wiring

Do **not** split by chain.
Do **not** split by contract file.
Do **not** put one dev on “SDK research” and another on “implementation” now — the research is already good enough.

For now, preferred plan:
- **1 implementer**
- **1 reviewer**

---

## Goal

Implement a small, durable, trustworthy Aave V3 integration in Heat.

Phase-1 target chains:
- Ethereum
- Arbitrum
- Base

Phase-1 target commands:
- `heat aave markets`
- `heat aave positions`
- `heat aave health`
- `heat aave supply`
- `heat aave withdraw`

Not in scope for this pass:
- borrow
- repay
- collateral toggles
- eMode changes
- rewards
- flash loans
- liquidation flows
- wrapped-native shortcuts
- giant market-management surfaces

Keep the first pass tight.

---

## Product and architecture rules

You must follow the project rules in:
- `CLAUDE.md`
- `PLAN.md`
- `ROADMAP.md`
- `CHECKLIST.md`
- `UPSTREAM.md`
- `REVIEW_GUIDE.md`

Important recurring rules:
- use `clap`
- keep parsing out of `heat-core`
- stdout = command data
- stderr = diagnostics only
- Heat owns the output contract
- use Heat DTOs, not upstream response structs as the CLI contract
- money-sensitive values should prefer strings in machine output
- dangerous writes need `--dry-run`, confirmation on TTY, and `--yes` override
- no interactive prompts in non-TTY mode
- account-first UX stays primary

---

## Read these files first

### Core Aave research
Read fully:
- `reference/aave-docs/AAVE_INTEGRATION_RESEARCH.md`

### Official Aave docs mirror
Read fully:
- `reference/aave-docs/official/overview.md`
- `reference/aave-docs/official/markets/overview.md`
- `reference/aave-docs/official/markets/data.md`
- `reference/aave-docs/official/markets/operations.md`
- `reference/aave-docs/official/markets/positions.md`
- `reference/aave-docs/official/smart-contracts/pool.md`
- `reference/aave-docs/official/smart-contracts/pool-addresses-provider.md`
- `reference/aave-docs/official/smart-contracts/view-contracts.md`
- `reference/aave-docs/official/resources/addresses.md`

### Contract/source references
Read fully:
- `reference/aave-v3-origin/src/contracts/interfaces/IPool.sol`
- `reference/aave-v3-origin/src/contracts/interfaces/IPoolAddressesProvider.sol`
- `reference/aave-v3-origin/src/contracts/interfaces/IPoolDataProvider.sol`
- `reference/aave-v3-origin/src/contracts/helpers/UiPoolDataProviderV3.sol`
- `reference/aave-v3-origin/src/contracts/helpers/interfaces/IUiPoolDataProviderV3.sol`
- `reference/aave-v3-origin/src/contracts/helpers/AaveProtocolDataProvider.sol`

### Official address-book references
Read fully:
- `reference/aave-docs/address-book/AaveV3Ethereum.sol`
- `reference/aave-docs/address-book/AaveV3Arbitrum.sol`
- `reference/aave-docs/address-book/AaveV3Base.sol`

### Heat substrate references
Read fully:
- `crates/heat-evm/src/chains.rs`
- `crates/heat-evm/src/rpc.rs`
- `crates/heat-evm/src/signer.rs`
- `crates/heat-evm/src/amount.rs`
- `crates/heat-evm/src/erc20.rs`
- `crates/heat-core/src/ctx.rs`
- `crates/heat-core/src/output.rs`
- `crates/heat-core/src/safety.rs`
- `crates/heat-cli/src/main.rs`

### Reference-only product/API material
Read for shape, not for architecture:
- `reference/aave-sdk/packages/client/src/actions/markets.ts`
- `reference/aave-sdk/packages/client/src/actions/user.ts`
- `reference/aave-sdk/packages/client/src/actions/transactions.ts`
- `reference/aave-sdk/packages/client/src/actions/reserve.ts`

Do **not** build Heat around the TS SDK.
It is reference material only.

---

## Required design decisions

These are already decided. Do not reopen them unless you find a real blocker.

### 1. Integration style
Use **contract-first Alloy integration**.

### 2. Canonical market entrypoint
Use **`PoolAddressesProvider`** as the canonical registry.

Do not treat hardcoded Pool addresses in runtime logic as the primary source of truth.

### 3. Read-path strategy
Use:
- `Pool.getUserAccountData(...)` for health/account summary
- `UiPoolDataProviderV3` for rich reserve/user market reads where practical
- `AaveProtocolDataProvider` for reserve config/token-address/user-reserve detail where useful

### 4. Write-path strategy
Use `Pool` directly for:
- `supply`
- `withdraw`

### 5. Supported chains
Only:
- Ethereum
- Arbitrum
- Base

### 6. Output policy
Use Heat-shaped DTOs and shared output modes.

No `Debug` output.
No raw upstream serialization leaks.
No “just print whatever the contract returned” shortcuts.

---

## Implementation plan

## Step 1 — add crate
Add a new crate:
- `crates/heat-aave`

Suggested files:
- `src/lib.rs`
- `src/cmd.rs`
- `src/addresses.rs`
- `src/contracts.rs`
- `src/read.rs`
- `src/write.rs`
- `src/dto.rs`
- `src/tests.rs`

You may split further if needed, but keep it small.

## Step 2 — wire CLI entry
Add top-level protocol wiring similar to existing protocol crates.

Expected command family:
- `heat aave markets`
- `heat aave positions`
- `heat aave health`
- `heat aave supply`
- `heat aave withdraw`

Use helper-first protocol UX, consistent with the rest of Heat.

## Step 3 — add market registry
In `addresses.rs`, define a small canonical registry for:
- market name / chain
- `PoolAddressesProvider`
- helper contract addresses when needed

Seed it from the locally stored official address-book files.

### Initial canonical addresses
#### Ethereum
- provider: `0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e`
- pool: `0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2`
- protocol data provider: `0x0a16f2FCC0D44FaE41cc54e079281D84A363bECD`
- ui pool data provider: `0x56b7A1012765C285afAC8b8F25C69Bf10ccfE978`

#### Arbitrum
- provider: `0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb`
- pool: `0x794a61358D6845594F94dc1DB02A252b5b4814aD`
- protocol data provider: `0x243Aa95cAC2a25651eda86e80bEe66114413c43b`
- ui pool data provider: `0x13c833256BD767da2320d727a3691BAff3770E39`

#### Base
- provider: `0xe20fCBdBfFC4Dd138cE8b2E6FBb6CB49777ad64D`
- pool: `0xA238Dd80C259a72e81d7e4664a9801593F98d1c5`
- protocol data provider: `0x0F43731EB8d45A581f4a36DD74F5f358bc90C73A`
- ui pool data provider: `0xb84A20e848baE3e13897934bB4e74E2225f4546B`

Use provider-first resolution in logic even if you also store the known pool/helper addresses for validation and tests.

## Step 4 — add minimal Alloy interfaces
In `contracts.rs`, define only the methods we need.

### `IPoolAddressesProvider`
Need at least:
- `getPool()`
- `getPoolDataProvider()`
- `getPriceOracle()`

### `IPool`
Need at least:
- `supply(address,uint256,address,uint16)`
- `withdraw(address,uint256,address)`
- `getUserAccountData(address)`

### `IPoolDataProvider`
Need only the minimum methods you actually use.
Likely:
- `getAllReservesTokens()`
- `getReserveConfigurationData(address)`
- `getReserveCaps(address)`
- `getPaused(address)`
- `getSiloedBorrowing(address)`
- `getReserveData(address)`
- `getUserReserveData(address,address)`
- `getReserveTokensAddresses(address)`

### `IUiPoolDataProviderV3`
Need only the minimum methods you actually use.
Likely:
- `getReservesData(address)`
- `getUserReservesData(address,address)`
- `getEModes(address)`

Do not paste giant ABI surfaces unless they are truly needed.

## Step 5 — implement read path first
Build these in order:

### `markets`
Goal: list supported reserves for the chosen chain.

Include useful machine fields such as:
- chain
- market
- underlying symbol
- underlying address
- decimals
- aToken address
- variable debt token address
- collateral enabled
- borrowing enabled
- paused/frozen/active flags
- supply cap
- borrow cap
- total supplied
- total variable debt
- supply APY
- variable borrow APY

Use strings for money-sensitive quantities.

### `positions`
Goal: show the selected account’s reserve-level position state.

Include fields like:
- chain
- account
- symbol
- asset address
- supplied
- borrowed
- collateral enabled
- aToken balance
- variable debt

### `health`
Goal: show account-level health summary.

Use `Pool.getUserAccountData(...)` as the primary source.

Include:
- chain
- account
- total collateral base
- total debt base
- available borrows base
- current liquidation threshold
- ltv
- health factor
- maybe eMode later if low-cost to include

Get read-path behavior solid before touching writes.

## Step 6 — implement write path
Only after read-path acceptance.

### `supply`
Requirements:
- resolve signer via `heat-evm`
- resolve user account/address via Heat account model
- resolve asset by symbol or address for the selected chain
- parse exact token amount safely
- check / set ERC20 approval using shared `heat-evm::erc20` helpers
- support `--dry-run`
- require confirmation in TTY unless `--yes`
- emit Heat-owned output

### `withdraw`
Requirements:
- same safety and account rules as `supply`
- allow exact token amount
- defer “withdraw max” unless implemented carefully

If you add “max”, it must be explicit and well-tested.
Do not guess.

---

## Chain and account rules

### Chain handling
Use Heat’s canonical chain handling from `heat-evm`.

Do not invent new chain naming.
Expected user-facing chain names:
- `ethereum`
- `arbitrum`
- `base`

### Account handling
Use the standard Heat account model.
Do not add Aave-specific key flags as the primary flow.

Preferred UX:
- `--account <NAME>`
- shared config/default account resolution

### RPC handling
Use shared RPC resolution from `heat-evm`.
Do not invent one-off Aave RPC env names unless absolutely necessary.

---

## Output contract requirements

All command output must follow Heat rules.

### Pretty mode
Human-optimized table/list/summary output.

### JSON mode
Raw Heat-shaped JSON payload.
No envelope unless Heat conventions already require one.

### NDJSON
Not needed unless you add a stream later.
Do not invent it now.

### Quiet mode
Only support it where there is a clear scalar result.
If there is no useful scalar output, reject or no-op consistently with Heat conventions.

### DTO guidance
Create explicit DTO structs in `dto.rs` for:
- market list / market row
- position row
- health summary
- supply result
- withdraw result

No debug serialization.
No accidental upstream type leakage.

---

## Safety rules for writes

This is real-money code.

### Required
- `--dry-run`
- clear preview before execution where practical
- confirmation on TTY unless `--yes`
- no interactive prompt in non-TTY
- fail loudly on chain mismatch / bad asset / bad amount

### Amount handling
Never use `f64` for execution logic.
Use exact decimal parsing with `heat-evm` amount helpers.
Reject over-precision.

### Approvals
For `supply`, approval behavior must be explicit.
If approval is needed:
- surface it clearly
- use shared ERC20 approval helper
- do not hide approval side effects

---

## Suggested acceptance order

### Milestone 1
- crate skeleton exists
- command tree wired
- addresses registry in place
- contracts compile

### Milestone 2
- `markets` works on Ethereum / Arbitrum / Base
- machine output is stable and Heat-shaped
- tests cover DTO mapping and asset lookup

### Milestone 3
- `positions` and `health` work
- account resolution is correct
- tests cover empty-account and normal-account paths

### Milestone 4
- `supply` works with approval flow and dry-run
- `withdraw` works with dry-run
- safety and confirmation behavior verified

Only after Milestone 4 should we consider phase-1 Aave “accepted enough”.

---

## Test expectations

At minimum, add tests for:
- chain/market registry lookup
- asset symbol lookup
- exact amount parsing behavior
- DTO serialization shape
- empty positions behavior
- health mapping behavior
- dry-run behavior for writes if feasible
- safety/confirmation behavior where unit-testable

If RPC-dependent tests are flaky, isolate them carefully or keep them out of default CI.
Prefer deterministic unit tests for mapping and validation.

---

## Common failure modes to avoid

Watch for these specifically:
- using the TS SDK as the runtime path
- hardcoding giant address maps without provider-first design
- printing raw contract tuples directly
- leaking `Debug` output into JSON mode
- silently falling back to the wrong chain
- weak asset-symbol matching that can pick the wrong reserve
- implicit approval side effects with poor user visibility
- using floating-point math for token amounts
- oversized ABI surfaces that are hard to review
- mixing clap parsing into shared core crates

---

## Deliverables expected from the developer

1. working `crates/heat-aave`
2. command wiring in the CLI
3. tests
4. docs/comments where needed for non-obvious decisions
5. updates to:
   - `CHECKLIST.md`
   - `ROADMAP.md` if scope/truth changed
   - `UPSTREAM.md` if new reference/version notes matter

If implementation changes assumptions, update docs immediately.

---

## Bottom line

Build a **small, contract-first, provider-first Aave integration**.

Use:
- `heat-evm`
- `PoolAddressesProvider`
- `Pool`
- `UiPoolDataProviderV3`
- `AaveProtocolDataProvider`

Deliver only:
- `markets`
- `positions`
- `health`
- `supply`
- `withdraw`

Do that well, keep output/safety/account behavior Heat-native, and stop.
