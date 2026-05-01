## Context

`execute_transfer` establishes the spend-policy enforcement pattern. `execute_swap` and `execute_lending_op` compose that same pattern with two additional CPI targets: Jupiter Aggregator v6 for DEX swaps, and Kamino (or Kamino-compatible) for lending. SPECIFICATION.md requires 10 bps fee on all three operation types. The backend exposes `/v1/swap`, `/v1/deposit`, `/v1/withdraw`; all three need on-chain enforcement or the policy model is incomplete.

## Goals / Non-Goals

**Goals:**
- `execute_swap`: identical enforcement order to `execute_transfer` (nonce → limits → whitelist) + Jupiter v6 CPI + fee deduction on `amount_in`.
- `execute_lending_op`: same enforcement + Kamino deposit/withdraw CPI + fee deduction (on principal for deposit, on redeemed amount for withdraw).
- Fee math reuses `util/fee.rs` exactly — no new bps logic.
- Both instructions share time/reset helpers from `util/time.rs`.
- All arithmetic checked; no panics.

**Non-Goals:**
- Building a generic DEX aggregator layer (Jupiter only, v1).
- Yield strategy selection / auto-compounding (single lend/deposit op per call).
- Frontend wallet CPI (operator-signed only, same as `execute_transfer`).
- Mainnet liquidity.

## Decisions

**`execute_swap` uses DEX router whitelist entry type-2 — not a hardcoded program ID.**
Rationale: the group owner sets the DEX router at `initialize_group` time. Hardcoding Jupiter's program ID in the program source makes it impossible to upgrade to a new aggregator version without a program upgrade. The type-2 whitelist entry lets the orchestrator rotate the router via `add_to_whitelist` / `remove_from_whitelist`.

**Fee deducted from agent ATA BEFORE the Jupiter CPI.**
Alternative: take fee from swap output. Rejected — output mint may differ from input mint; fee wallet holds USDC; taking fee from output would require a second swap leg and is complex. Input-mint fee is simpler, deterministic, and what the spec says ("10 bps from every outbound transfer and swap").

**Jupiter CPI via `remaining_accounts`.**
Jupiter v6 accepts a variable-length account list for route legs. Passing via `remaining_accounts` rather than a fixed `Accounts` struct avoids needing to enumerate every possible route shape. The Accounts struct only pins: `jupiter_program`, `agent_wallet` (signer seeds), `from_token_account`, `to_token_account`, `token_program`.

**`execute_lending_op` constrains lending program to type-2 whitelist entry.**
Prevents operator from CPIing into arbitrary programs under the guise of "lending". Only programs that the orchestrator has explicitly whitelisted as type-2 can be used.

**Deposit fee on principal in; withdraw fee on redeemed amount out.**
Consistent with "10 bps on every outbound agent-wallet token movement." For deposit, agent sends `principal` out (fee deducted before sending). For withdraw, agent receives `redeemed` in; fee is taken from that inbound before it lands in the agent ATA — net landing = `redeemed - fee`.

**Spend-limit enforcement applies to gross `amount_in` (same as execute_transfer).**
Gross counting prevents splitting large swaps into micro-operations to evade daily/hourly caps.

**`execute_lending_op` `op_type` arg: 0 = deposit, 1 = withdraw.**
Simple u8 discriminant keeps a single instruction entry point; avoids routing complexity in `lib.rs`.

## Risks / Trade-offs

- [Jupiter v6 interface change breaks CPI] → Pin Jupiter program ID as a constant; CI smoke test catches breakage. Upgrade path: orchestrator updates router whitelist entry to new program, redeploy if program ID changes.
- [Kamino lend interface variation] → Use the minimal shared interface (deposit/redeem with fixed accounts). Tested against devnet Kamino fork.
- [Fee-before-swap changes effective slippage] → Document in `docs/SECURITY_REVIEW.md`. Backend should quote `amount_in - fee` to Jupiter, not `amount_in`, so slippage tolerance applies to net amount.
- [Redeemed amount less than fee on tiny withdraw] → `checked_sub` returns error → `InvalidAmount`; handler rejects the withdraw rather than draining more than received.

## Migration Plan

Additive — no migration. Deploys via the same `anchor deploy` cycle. Backend enables `/v1/swap`, `/v1/deposit`, `/v1/withdraw` only after this change lands on devnet and smoke test passes.

## Open Questions

- Whether to support multi-hop swaps (multiple Jupiter legs in one tx) — defer; single-leg first.
- Whether to auto-reinvest yield (compound) — out of scope for v1; separate `execute_compound` change later.
