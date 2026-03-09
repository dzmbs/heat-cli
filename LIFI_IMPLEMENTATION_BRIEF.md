# LIFI_IMPLEMENTATION_BRIEF.md

This brief is for the developer implementing LI.FI as a publishable Heat feature.

Read this before writing code.

---

## Product decision

LI.FI is now a **core Heat feature**.

The goal is to make `heat lifi` good enough to publish as Heat's EVM capital-movement layer.

That means:
- real bridge/routing workflows
- real execution for a narrow supported scope
- Heat-native safety, output, account, and RPC behavior
- no pretending that a quote-only stub is enough

---

## Staffing recommendation

Use **one developer** for implementation.

Reason:
- the execution path must stay coherent across:
  - route discovery
  - step-transaction retrieval
  - approvals
  - transaction sending
  - status tracking
  - safety
- splitting too early risks inconsistent execution semantics and output DTO drift

Best setup:
- **1 implementer**
- **1 reviewer**

If schedule pressure forces a split, the only acceptable boundary is:
- **Dev A:** read/discovery (`chains`, `tokens`, `tools`, `quote`, `routes`, DTOs)
- **Dev B:** execution (`bridge`, `execute`, approvals, step transactions, status, safety)

Do not split by chain.
Do not split by HTTP client vs CLI.

---

## Scope freeze: what LI.FI means in Heat v1

### Core user job
Move capital safely between supported EVM chains.

### Supported chain scope for publishable v1
Required:
- Ethereum
- Polygon
- Arbitrum
- Base

Optional only if verified cleanly through LI.FI `/chains` and execution works cleanly:
- Optimism

Not in scope for v1 promise:
- HyperCore
- Solana
- non-EVM routes
- arbitrary chain families

HyperEVM may only be added if:
- LI.FI officially supports it in `/chains`
- route execution works through the same EVM execution path
- tests/docs are updated explicitly

Do **not** promise HyperEVM by assumption.

### Supported asset scope for publishable v1
Required first-pass support:
- native gas token bridges where LI.FI supports them on the chosen route
- major stablecoins by token address/symbol where supported:
  - USDC
  - USDT
  - DAI
- WETH/ETH if route support is clean

Do not market “all tokens on all chains”.

### Supported user flows
Publishable v1 must support:
- route discovery
- a clear bridge helper command
- execution of supported EVM→EVM routes
- route/transfer status inspection

---

## Required command surface

Keep existing protocol-native reads, but add a Heat-native helper flow.

### Must exist
- `heat lifi chains`
- `heat lifi tokens`
- `heat lifi tools`
- `heat lifi quote`
- `heat lifi routes`
- `heat lifi status`
- `heat lifi bridge`

### Preferred execution UX
```bash
heat lifi bridge 1000 USDC --from ethereum --to polygon --account main
heat lifi bridge 0.5 ETH --from base --to arbitrum --account main
heat lifi status --tx-hash 0x... --from-chain 8453 --to-chain 42161 --bridge across
```

### `bridge` command requirements
`bridge` should:
1. resolve source/destination chains using Heat chain names
2. resolve token input safely
3. resolve sender address from Heat account
4. request routes from LI.FI
5. choose one route deterministically (see selection rules below)
6. dry-run / preview if requested
7. confirm dangerous execution on TTY unless `--yes`
8. execute route step-by-step for supported EVM-only routes
9. emit Heat-owned result DTOs

### Route selection rules
First pass should be explicit and predictable.

Support:
- default: LI.FI recommended/best route from returned ordering
- `--route-index <N>` to select a specific route from `routes`

Do not invent fuzzy scoring logic.
Do not silently pick a weird route if the top route is unsupported.
If no returned route is executable by Heat, fail clearly.

---

## Architecture decisions

These are already decided. Do not reopen them without a real blocker.

### 1. Integration style
Use **direct LI.FI REST integration in Rust**.

Do not make the TypeScript SDK the runtime path.
The SDK is reference material only.

### 2. Execution substrate
Use `heat-evm` for:
- chain naming
- RPC resolution
- signer resolution
- amount parsing
- approval flow
- sending EVM transactions

LI.FI plans the route.
Heat executes the route.

### 3. Heat owns the output contract
Never expose raw LI.FI JSON as the public CLI contract.
Use Heat DTOs.

### 4. Execution scope
For publishable v1, execute only:
- EVM → EVM routes
- where every step can be executed through Heat's EVM path

Reject unsupported route families clearly.

### 5. Safety policy
Execution is dangerous and real-money affecting.

Must support:
- `--dry-run`
- TTY confirmation unless `--yes`
- no prompts in non-TTY
- explicit approvals surfaced before execution
- explicit failure if a route requires capabilities we do not support yet

---

## Read these files first

### Current Heat LI.FI implementation
Read fully:
- `crates/heat-lifi/src/lib.rs`
- `crates/heat-lifi/src/cmd.rs`
- `crates/heat-lifi/src/client.rs`
- `crates/heat-lifi/src/dto.rs`
- `crates/heat-lifi/src/map.rs`
- `crates/heat-lifi/src/exec.rs`
- `crates/heat-lifi/src/tests.rs`
- `crates/heat-lifi/src/http_tests.rs`

### Heat EVM substrate
Read fully:
- `crates/heat-evm/src/chains.rs`
- `crates/heat-evm/src/rpc.rs`
- `crates/heat-evm/src/signer.rs`
- `crates/heat-evm/src/amount.rs`
- `crates/heat-evm/src/erc20.rs`

### Heat core contracts
Read fully:
- `crates/heat-core/src/ctx.rs`
- `crates/heat-core/src/output.rs`
- `crates/heat-core/src/safety.rs`
- `crates/heat-core/src/error.rs`

### LI.FI references
Read fully:
- `reference/lifi-sdk/README.md`
- `reference/lifi-sdk/packages/sdk/src/actions/getQuote.ts`
- `reference/lifi-sdk/packages/sdk/src/actions/getRoutes.ts`
- `reference/lifi-sdk/packages/sdk/src/actions/getStatus.ts`
- `reference/lifi-sdk/packages/sdk/src/actions/getStepTransaction.ts`
- `reference/lifi-sdk/packages/sdk/src/core/execution.ts`
- `reference/lifi-sdk/examples/node/examples/bridge.ts`
- `reference/lifi-sdk/examples/node/examples/swap.ts`

### Project rules
Read fully:
- `CLAUDE.md`
- `PLAN.md`
- `ROADMAP.md`
- `CHECKLIST.md`
- `UPSTREAM.md`
- `REVIEW_GUIDE.md`

---

## Exact implementation goals

## Phase A — clean up the current read layer
The current `heat-lifi` crate describes itself as read-only and execution-deferred.
That must be updated carefully, not hacked around.

### Keep and harden
- `chains`
- `tokens`
- `tools`
- `quote`
- `routes`
- `status`

### Improve input UX
Current commands still use numeric chain IDs in places.
For Heat publishable UX, support canonical Heat chain names at the CLI level:
- `ethereum`
- `polygon`
- `arbitrum`
- `base`
- optional `optimism`

Internally convert to the LI.FI numeric chain IDs.

Numeric IDs may still be accepted as power-user input, but chain names must be first-class.

### Token resolution rules
At minimum support:
- explicit token addresses
- canonical native token alias (`ETH` on EVM)
- major stablecoin symbols only when unambiguous on the selected chain

Do not do loose global symbol matching across chains.
Use LI.FI `/tokens` for the specific chain.

---

## Phase B — add execution primitives
Add what is necessary to execute supported EVM-only routes.

### Client layer
Extend `client.rs` to support:
- `POST /advanced/stepTransaction`

If needed, add the raw response/request types for step transaction payloads.

Do not over-model the whole LI.FI schema. Model only the parts Heat executes.

### Execution module
Create or extend execution code so that Heat can:
1. inspect a chosen route
2. iterate route steps in order
3. request transaction data for each step from LI.FI
4. decide whether approval is needed before step execution
5. send approval if required
6. send the route step transaction
7. collect tx hashes / step results
8. fail loudly on unsupported step contents

If the current `exec.rs` only classifies routes, expand it or add a dedicated execution module.

### Supported step model
For publishable v1, support steps where LI.FI returns normal EVM transaction requests that can be executed through `heat-evm` wallet providers.

Reject steps that require:
- unsupported chain families
- unsupported signing model
- custom wallet callbacks / browser-only assumptions
- features we cannot safely reproduce in CLI form

---

## Phase C — bridge helper command
Implement:
- `heat lifi bridge <AMOUNT> <TOKEN> --from <CHAIN> --to <CHAIN>`

Suggested fields:
- amount: human-readable amount
- token: source token symbol/address
- `--to-token <TOKEN>` optional, default same logical token if supported
- `--from <CHAIN>` required unless `--network` fallback is intentionally supported
- `--to <CHAIN>` required
- `--route-index <N>` optional
- `--rpc <URL>` optional source-chain override if needed

### Bridge command behavior
1. resolve source chain + destination chain
2. resolve account/sender address from Heat account
3. resolve source token on source chain
4. resolve destination token
5. parse amount exactly using source token decimals
6. request routes
7. filter to routes Heat can execute today
8. choose route deterministically
9. dry-run preview if requested
10. confirm dangerous execution unless `--yes`
11. execute route steps
12. output Heat-owned result DTO

### Dry-run preview must include
- from chain / to chain
- source token / destination token
- input amount
- expected output amount and minimum output
- tools/bridges used
- number of steps
- whether approval will be needed
- route tags if present

---

## Execution details and safety rules

### Approval handling
For steps that require ERC-20 approval:
- surface approval explicitly to the user
- use shared `heat-evm::erc20` helpers where possible
- if the route requires approval reset patterns or step-specific spender addresses, make them visible in diagnostics/output
- do not hide approvals inside vague “executing route” text

### Transaction sending
Use Heat EVM wallet providers and send normal EVM txs.

All execution must:
- verify chain alignment
- use exact calldata/value from LI.FI step transaction payload
- treat tx failures as hard errors
- capture transaction hashes per step

### Non-TTY behavior
No interactive prompts.
If confirmation is required and `--yes` is missing in non-TTY, fail clearly.

### Zero/fake execution
Do not claim `bridge` execution support if it only fetches quotes.
Execution must be real or the command must remain read-only.

---

## Supported output contracts

Add Heat-owned DTOs for:
- bridge preview
- bridge execution result
- route step execution result
- route selection summary (if helpful)

### Machine output guidance
Money-sensitive values must stay strings.
Include fields like:
- source chain
- destination chain
- source token
- destination token
- from amount
- to amount estimate
- to amount minimum
- route id
- route tags
- steps[]
- execution_supported
- step transaction hashes
- approval transaction hashes
- final status summary

### Pretty output guidance
Pretty output should read like a clear operator preview, not a raw API dump.

---

## Suggested crate/file changes

Current crate likely needs at least:
- `src/client.rs` — add stepTransaction support and any execution request types
- `src/cmd.rs` — add `bridge`, improve chain-name UX
- `src/dto.rs` — add execution/preview DTOs
- `src/exec.rs` — real execution logic, not just classification
- `src/map.rs` — keep mapping clean and deterministic
- `src/tests.rs` / `src/http_tests.rs` — add execution-path tests

If needed, add:
- `src/resolve.rs` — chain/token resolution helpers
- `src/execute.rs` — if execution logic gets too large for `exec.rs`

Keep it practical. Do not over-abstract.

---

## Acceptance criteria for publishable LI.FI

LI.FI is not done because it can print routes.
It is done when:

### Read layer
- `chains`, `tokens`, `tools`, `quote`, `routes`, `status` are stable
- chain-name UX is Heat-native
- JSON output is Heat-shaped and clean

### Execution layer
- `bridge` executes supported EVM-only routes for the scoped chains
- approvals are explicit and safe
- dry-run is useful
- confirmation behavior is correct
- route step txs are actually sent and tracked
- unsupported routes fail early and clearly

### Product honesty
- docs/help reflect the real supported chain/token scope
- no fake “all chains/all tokens” implication
- no claiming HyperCore/non-EVM routing support

### Quality
- unit tests cover mapping and validation
- HTTP tests cover LI.FI request/response handling
- execution-path tests cover step transaction handling and unsupported-step rejection
- `cargo clippy --all-targets -- -D warnings` passes

---

## Common failure modes to avoid

Watch for these specifically:
- keeping numeric-chain UX as the main public interface
- claiming execution support while still being quote-only underneath
- hiding approvals from the user
- sending transactions on the wrong chain due to route/step mismatch
- assuming every returned LI.FI route is executable by Heat
- loose token-symbol matching that chooses the wrong token
- raw LI.FI JSON leaking into output
- “best effort” errors that should be hard failures in a money-moving command
- trying to support non-EVM routes in the first publishable slice
- promising HyperEVM or HyperCore without explicit verification

---

## Deliverables expected from the developer

1. publishable `heat-lifi` read + execution flow
2. real `bridge` command
3. tests
4. updates to:
   - `CHECKLIST.md`
   - `ROADMAP.md` if scope/truth changed
   - `UPSTREAM.md` if new API notes/decisions matter
5. docs/help text reflecting the actual supported scope

---

## Bottom line

Build LI.FI as Heat's **production-ready EVM bridge layer** for a narrow, explicit scope.

Required publishable scope:
- Ethereum
- Polygon
- Arbitrum
- Base
- read commands
- real `bridge` execution
- real `status`
- Heat-native safety/output/account behavior

Do that well, keep it honest, and stop.
