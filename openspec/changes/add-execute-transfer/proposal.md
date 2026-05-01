## Why

`execute_transfer` is the entire on-chain enforcement surface of Enclz — the one instruction that actually moves agent funds and applies the policy ceiling. It is the product. Backend can fully provision groups today (after `add-owner-instructions`), but cannot move a single token until this lands. We split it from owner instructions because the logic is large (12-step ordered enforcement), failure modes are many, and it deserves a dedicated review pass.

## What Changes

- New backend-operator-signed instruction `execute_transfer(amount: u64, expected_nonce: u64)` in `programs/enclz/`.
- Strict ordered enforcement per SPECIFICATION.md §execute_transfer:
  1. Verify `expected_nonce == agent_wallet.operator_nonce` → reject `NonceMismatch`.
  2. Increment `operator_nonce`.
  3. Roll `spent_today` / `tx_count_this_hour` if the on-chain `Clock` crossed UTC midnight or the hour boundary.
  4. Reject if `amount > per_tx_limit`.
  5. Reject if `spent_today + amount > daily_limit`.
  6. Reject if `tx_count_this_hour >= hourly_tx_cap`.
  7. Verify `whitelist_entry` PDA exists for recipient → reject `WhitelistViolation`.
  8. If `entry_type == 1`: reject `WhitelistExpired` if `now > ttl_expires_at`; reject `WhitelistAmountExhausted` if `amount_used + amount > approved_amount`.
  9. Compute `protocol_fee = amount * 10 / 10_000`, `net = amount - fee` (10 bps).
  10. CPI two `token::transfer`: `net` to recipient ATA, `protocol_fee` to fee wallet ATA.
  11. Increment `spent_today += amount` (gross, fee counts), `tx_count_this_hour += 1`.
  12. If `entry_type == 1`: increment `amount_used += amount`; if exhausted, close PDA and return rent to owner.
- Helper module `util/time.rs` for daily/hourly reset detection using `Clock::get()`.
- Helper module `util/fee.rs` for the basis-point calculation.
- Comprehensive test suite covering every reject branch + auto-void + time travel.

## Capabilities

### New Capabilities
- `transfer-execution`: the `execute_transfer` instruction, fee math, time-window resets, whitelist consumption, auto-void

### Modified Capabilities
<!-- none — purely additive -->

## Impact

- Adds `programs/enclz/src/instructions/execute_transfer.rs` + `util/time.rs` + `util/fee.rs`.
- Wires `execute_transfer` entry point in `lib.rs`.
- Mutates `AgentWallet` (counters, nonce) and may close `WhitelistEntry` (auto-void).
- Backend can now serve the agent REST `/v1/transfer` endpoint end-to-end.
- All `checked_add` / `checked_mul` arithmetic — no `unwrap`.
- Depends on `init-anchor-workspace` + `add-owner-instructions` (needs PDAs to exist).
