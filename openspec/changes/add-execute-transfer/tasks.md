## 1. Helpers

- [ ] 1.1 Create `programs/enclz/src/util/mod.rs`
- [ ] 1.2 Create `util/time.rs` with `needs_daily_reset(last_reset, now) -> bool` and `needs_hourly_reset(last_reset, now) -> bool` using UTC midnight / hour boundaries
- [ ] 1.3 Create `util/fee.rs` with `compute_fee(amount: u64) -> Result<(u64, u64)>` returning `(net, fee)` using `PROTOCOL_FEE_BPS`, all checked arithmetic
- [ ] 1.4 Unit-test both helpers (boundary cases, overflow)

## 2. Instruction implementation

- [ ] 2.1 Create `programs/enclz/src/instructions/execute_transfer.rs` with `Accounts` struct: `backend_operator` (signer), `group_config` (with `has_one = backend_operator` constraint), `agent_wallet` (writable), `from_token_account`, `to_token_account`, `whitelist_entry` (seeds-checked), `protocol_fee_token_account`, `token_program`, `system_program`
- [ ] 2.2 Implement handler step 1: nonce check + early reject `NonceMismatch`
- [ ] 2.3 Implement handler step 2: increment `operator_nonce` (`checked_add`)
- [ ] 2.4 Implement handler step 3: invoke `time::needs_*_reset` helpers; zero counters when needed; update `last_*_reset` timestamps
- [ ] 2.5 Implement handler steps 4–6: per-tx, daily, hourly checks
- [ ] 2.6 Implement handler step 7: whitelist PDA seed + existence verified by Anchor account constraint (handler returns `WhitelistViolation` on missing-account error)
- [ ] 2.7 Implement handler step 8: type-1 TTL + `amount_used + amount > approved_amount` checks
- [ ] 2.8 Implement handler step 9: call `fee::compute_fee(amount)`
- [ ] 2.9 Implement handler step 10: two `token::transfer` CPIs signed with agent PDA seeds (`["wallet", group, idx, bump]`)
- [ ] 2.10 Implement handler step 11: increment `spent_today` (gross) + `tx_count_this_hour` with `checked_add`
- [ ] 2.11 Implement handler step 12: type-1 increment `amount_used`; close PDA via `close = owner` constraint applied conditionally when exhausted (use `AccountInfo::reload` or manual lamport transfer + assign owner pattern)
- [ ] 2.12 Wire entry point in `lib.rs`

## 3. Tests — happy paths

- [ ] 3.1 LiteSVM: transfer to type-0 (intra-group) recipient succeeds; net + fee land in correct ATAs; counters increment
- [ ] 3.2 LiteSVM: transfer to type-1 (external) recipient succeeds; `amount_used` increments
- [ ] 3.3 LiteSVM: transfer to type-2 (protocol) recipient succeeds; no `amount_used` change
- [ ] 3.4 LiteSVM: fee math — `amount = 1_000_000` → `net = 999_000`, `fee = 1_000`
- [ ] 3.5 LiteSVM: `spent_today` counts gross, not net

## 4. Tests — reject paths (one per requirement)

- [ ] 4.1 LiteSVM: stale nonce → `NonceMismatch`; no other state mutated
- [ ] 4.2 LiteSVM: replay (same nonce twice) → second call `NonceMismatch`
- [ ] 4.3 LiteSVM: `amount > per_tx_limit` → `PerTxLimitExceeded`
- [ ] 4.4 LiteSVM: `spent_today + amount > daily_limit` → `DailyLimitExceeded`
- [ ] 4.5 LiteSVM: `tx_count_this_hour == hourly_tx_cap` → `HourlyCapExceeded`
- [ ] 4.6 LiteSVM: missing whitelist entry → `WhitelistViolation`
- [ ] 4.7 LiteSVM: type-1 entry past TTL → `WhitelistExpired`
- [ ] 4.8 LiteSVM: type-1 entry over cap → `WhitelistAmountExhausted`
- [ ] 4.9 LiteSVM: non-operator signer → `Unauthorized`
- [ ] 4.9.1 LiteSVM: after `update_backend_operator`, the previously-valid operator's `execute_transfer` fails with `Unauthorized` (handed off from `add-owner-instructions` task 4.9 — rotation itself is verified there; this asserts the rotation actually invalidates the old key)
- [ ] 4.10 LiteSVM: arithmetic overflow → `InvalidAmount`
- [ ] 4.11 LiteSVM: `from_token_account.owner != agent_wallet` → constraint rejection
- [ ] 4.12 LiteSVM: non-USDC mint on `from_token_account` → constraint rejection
- [ ] 4.13 LiteSVM: `protocol_fee_token_account.owner != group_config.protocol_fee_wallet` → constraint rejection
- [ ] 4.14 LiteSVM: `to_token_account.owner` does not match whitelist PDA seed target → constraint rejection
- [ ] 4.15 Property test: for any `amount` in `1..=u64::MAX/2`, `net + fee == amount` (verify `compute_fee` is lossless)

## 5. Tests — time travel + auto-void

- [ ] 5.1 LiteSVM: advance clock past UTC midnight → `spent_today` resets on next call
- [ ] 5.2 LiteSVM: advance clock past hour boundary → `tx_count_this_hour` resets on next call
- [ ] 5.3 LiteSVM: type-1 transfer that exhausts `approved_amount` → PDA closed, rent returns to owner
- [ ] 5.4 LiteSVM: post-auto-void transfer → `WhitelistViolation` (not `WhitelistAmountExhausted`)
- [ ] 5.5 LiteSVM: after auto-void, orchestrator re-creates whitelist entry via `add_to_whitelist` → succeeds; next transfer succeeds under new cap

## 6. Integration tests

- [ ] 6.1 Mocha: end-to-end against `solana-test-validator` — provision group + agent, fund agent ATA, register external whitelist with $5 cap, execute transfers totaling $5, assert PDA closed, attempt 6th transfer → `WhitelistViolation`
- [ ] 6.2 Mocha: replay protection — submit two txs with same nonce concurrently, assert exactly one succeeds

## 7. Verification

- [ ] 7.1 `cargo test --package enclz`: all unit tests green
- [ ] 7.2 `anchor test`: integration green
- [ ] 7.3 Coverage on `execute_transfer.rs` ≥ 90% (per spec, this is the highest-priority code)
- [ ] 7.4 Manual review: walk steps 1–12 against handler source, line by line
