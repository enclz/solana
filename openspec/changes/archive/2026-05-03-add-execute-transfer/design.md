## Context

This is the highest-stakes instruction in the program. A bug here = lost funds, bypassed policy, or a stuck agent. SPECIFICATION.md prescribes the exact 12-step enforcement order; deviation breaks the security model (e.g., checking whitelist before nonce would let a replay attacker probe the whitelist with no cost). Every step needs both a passing and a failing test.

## Goals / Non-Goals

**Goals:**
- Single instruction implementing all 12 steps in spec order.
- Every reject path returns the spec-mandated error variant — backend translates it verbatim to REST error code.
- Replay protection: nonce check + increment happen before any other state mutation.
- Auto-void: when a type-1 entry hits its cap, close the PDA atomically in the same instruction so subsequent transfers fail with `WhitelistViolation` (not `WhitelistAmountExhausted`).
- Time resets are deterministic from `Clock::get()` — no off-chain dependency.
- All arithmetic checked; overflow → `InvalidAmount` rather than panic.

**Non-Goals:**
- Swap / deposit / withdraw flows (separate change later — those compose `execute_transfer` for the fee leg but route the principal through Jupiter / lending CPIs).
- Backend simulation endpoint (mirrors this logic off-chain in JS).
- Webhook dispatch (off-chain).

## Decisions

**Order matters; encode it as a linear handler, not split helpers.**
A reader auditing the security boundary needs to see steps 1→12 in one place. Helpers only for `time` (reset detection) and `fee` (bps math) — both pure functions.

**Nonce check first, before any reads.**
If we whitelist-checked first, an attacker replaying old txs could probe whitelist state cheaply. Nonce-first means stale txs fail immediately with no other state touched.

**Increment nonce before validation, not after.**
If validation fails the tx aborts and state rolls back — the nonce increment also rolls back. So "increment before check" is equivalent to "increment on success only" but written as one unconditional line.

**`spent_today += amount` (gross, including fee), not `net_amount`.**
Spec says fee counts against the daily limit. Otherwise an agent could spam micro-transfers, paying only fees and never tripping the cap.

**Auto-void closes the PDA in the same instruction, returning rent to the orchestrator.**
Alternative: leave PDA open with `amount_used >= approved_amount` and reject in step 8. Rejected — leaves dust PDAs around, costs the orchestrator rent indefinitely, and forces them to manually clean up.

**`Clock::get().unix_timestamp` for time, not block height.**
Spec uses Unix timestamps for TTL and resets. Solana clock drift is bounded enough for daily/hourly windows.

**Whitelist entry seed derives from `to_token_account.owner`, not a separate `recipient` arg.**
```rust
#[account(seeds = [b"whitelist", group_config.key().as_ref(), to_token_account.owner.as_ref()], bump)]
pub whitelist_entry: Account<'info, WhitelistEntry>,
```
Critical: if seed used an independent `recipient: Pubkey` arg, an attacker could pair a valid whitelist PDA with a `to_token_account` whose owner is a non-whitelisted address, draining funds to an un-whitelisted wallet. Anchoring the seed to the ATA's owner field eliminates that vector entirely.

**`protocol_fee_token_account` constrained to match `group_config.protocol_fee_wallet` and USDC mint.**
```rust
#[account(
    mut,
    constraint = protocol_fee_token_account.owner == group_config.protocol_fee_wallet,
    constraint = protocol_fee_token_account.mint == from_token_account.mint,
)]
pub protocol_fee_token_account: Account<'info, TokenAccount>,
```
Without this, an attacker could pass their own ATA and reroute the 10 bps fee to themselves.

**`from_token_account` constrained to be owned by the `agent_wallet` PDA with consistent mint.**
```rust
#[account(
    mut,
    constraint = from_token_account.owner == agent_wallet.key(),
    constraint = from_token_account.mint == to_token_account.mint,
)]
pub from_token_account: Account<'info, TokenAccount>,
```
Anchor `Token::Account` does not enforce owner by default; explicit constraint required.

**Mint consistency enforced; absolute USDC pin deferred.**
The handler enforces `from_token_account.mint == to_token_account.mint == protocol_fee_token_account.mint` (see the constraint code above). It does **not** pin the mint to a hardcoded `USDC_MINT` constant — that would require fixture infrastructure (preloaded mint at a known mainnet address) for both LiteSVM and `solana-test-validator`-based tests, and the consistency rule already eliminates the security-critical attack class (rerouting fees to a different-mint ATA the attacker controls). The orchestrator's choice of mint at `add_agent` time becomes the agent's effective operating mint; for v1 that's always USDC by convention, and the spend-limit accounting is denominated in 6-decimal units to match. Adding an absolute USDC mint pin is tracked as a future hardening change.

**Reset boundary uses clock-aligned windows, not sliding.**
```rust
fn needs_daily_reset(last_reset: i64, now: i64) -> bool { now / 86400 > last_reset / 86400 }
fn needs_hourly_reset(last_reset: i64, now: i64) -> bool { now / 3600 > last_reset / 3600 }
```
"UTC midnight" = day index change; "on the hour" = hour index change. A sliding window would let an agent accumulate more than the daily limit across two consecutive periods.

**Conditional auto-void via manual lamport zero-out, not Anchor `close =`.**
Anchor's `close = receiver` runs unconditionally at account resolution. A conditional close (only when `amount_used >= approved_amount`) requires:
1. Transfer lamports: `**from_info.lamports.borrow_mut() = 0; **to_info.lamports.borrow_mut() += rent;`
2. Reassign: `from_info.assign(&System::id())`
3. Zero data: `from_info.data.borrow_mut().fill(0)`
This must happen at the end of the handler, after all other state mutations succeed.

**Two SPL `token::transfer` CPIs (not one with split logic).**
Cleaner audit, lets each call use the agent's PDA signer seeds independently. Net first, then fee — if fee leg fails the whole tx aborts and net rolls back.

## Risks / Trade-offs

- [Reset logic edge case at exact midnight UTC] → Test asserts boundary: a tx at `last_reset == midnight - 1s` followed by a tx at `midnight + 1s` resets `spent_today`. Use LiteSVM clock travel.
- [Auto-void when `amount_used + amount` overflows] → `checked_add` returns `None` → reject as `InvalidAmount` rather than panic. Exhaustion check uses `>=` after the add, not `==`.
- [Fee rounding favors agent or protocol] → `amount * 10 / 10_000` truncates toward zero (favors agent on tiny amounts). Documented; for $0.01 agents the protocol forfeits sub-cent fees, acceptable.
- [`Clock::get` failure in CPI context] → Wrapped in `?`; surfaces as `internal_error` REST-side.
- [Nonce overflow at u64::MAX] → Acknowledged but unreachable in practice (≈10^19 transfers per agent).

## Migration Plan

Additive. Deploys via the same `anchor deploy` cycle as previous changes. Backend should keep `/v1/transfer` returning a "not yet available" error until this lands on devnet, then enable the route.

## Open Questions

- Whether to emit an Anchor event (`TransferExecuted`) for indexer / webhook efficiency — defer; current backend uses RPC log subscription which captures CPI logs.
- Whether the fee wallet ATA must be passed pre-existing or can be created on-demand — pre-existing for v1 (orchestrator funds it once at group init); creating on-demand would add a CPI on every transfer.
