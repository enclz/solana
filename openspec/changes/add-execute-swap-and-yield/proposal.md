## Why

SPECIFICATION.md mandates a flat 10 basis-point protocol fee on every outbound **transfer, swap, and yield operation**. The three prior changes implement `execute_transfer`; without `execute_swap` and `execute_lending_op`, the backend cannot implement `/v1/swap`, `/v1/deposit`, or `/v1/withdraw`, and those routes return "not yet available" indefinitely. This change closes that gap by adding two instructions that apply the same spend-policy + whitelist + fee enforcement as `execute_transfer`, then route the principal through Jupiter v6 (swap) and a Kamino-compatible lending program (yield).

## What Changes

- `programs/enclz/src/instructions/execute_swap.rs` — operator-signed instruction that:
  - Enforces nonce, time resets, per-tx/daily/hourly limits, and DEX-router whitelist (entry_type=2) using the same 12-step order as `execute_transfer`
  - Computes `protocol_fee = amount_in * 10 / 10_000`; deducts fee from agent ATA before swap
  - CPIs into Jupiter Aggregator v6 with the remaining `net_amount_in`
  - Returns any leftover tokens to the agent ATA (slippage remainder)

- `programs/enclz/src/instructions/execute_lending_op.rs` — operator-signed instruction that:
  - Enforces the same nonce + spend-policy checks
  - `op_type: u8` selects `deposit` (0) or `withdraw` (1)
  - For deposit: deducts fee from principal, CPIs into Kamino lend/deposit with `net_principal`
  - For withdraw: CPIs to redeem, then deducts fee from redeemed amount before crediting agent ATA
  - Lending program address constrained to a `WhitelistEntry` with `entry_type == 2`

- Tests covering: happy swap, happy deposit/withdraw, fee deduction correctness, spend-limit enforcement, whitelist-type enforcement (must be type-2 for DEX/lending), replay protection
- New npm scripts: `"test:swap": "anchor test --skip-build -- --grep swap"`, `"test:lending": "anchor test --skip-build -- --grep lending"`

## Capabilities

### New Capabilities
- `swap-execution`: Jupiter v6 CPI with spend-policy + fee enforcement
- `lending-execution`: Kamino deposit/withdraw CPI with spend-policy + fee enforcement

### Modified Capabilities
<!-- none — additive -->

## Impact

- Adds `instructions/execute_swap.rs`, `instructions/execute_lending_op.rs` to the existing Anchor program.
- Modifies `lib.rs` to wire two new entry points.
- Depends on `add-execute-transfer` (shared helpers: `util/time.rs`, `util/fee.rs`, account constraints pattern).
- Depends on `add-owner-instructions` for the type-2 whitelist entries (DEX router + lending program).
- Backend can now implement `/v1/swap`, `/v1/deposit`, `/v1/withdraw` against the deployed program.
- Mainnet deploy remains out of scope.
