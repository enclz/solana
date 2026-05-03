## 1. Helpers

- [x] 1.1 Create `programs/enclz/src/util/mod.rs`
- [x] 1.2 Create `util/time.rs` with `needs_daily_reset(last_reset, now) -> bool` and `needs_hourly_reset(last_reset, now) -> bool` using UTC midnight / hour boundaries
- [x] 1.3 Create `util/fee.rs` with `compute_fee(amount: u64) -> Result<(u64, u64)>` returning `(net, fee)` using `PROTOCOL_FEE_BPS`, all checked arithmetic
- [x] 1.4 Unit-test both helpers (boundary cases, overflow)

## 2. Instruction implementation

- [x] 2.1 Create `programs/enclz/src/instructions/execute_transfer.rs` with `Accounts` struct: `backend_operator` (signer), `group_config` (with `has_one = backend_operator` constraint), `group_owner` (writable, address-bound to `group_config.owner` — receives auto-void rent), `agent_wallet` (writable), `from_token_account`, `to_token_account`, `whitelist_entry` (seeds-checked against `to_token_account.owner`), `protocol_fee_token_account`, `token_program`, `system_program`. Token accounts wrapped in `Box` to keep the struct off the stack (the unboxed form overflows the 4 KB BPF stack frame; verified empirically — see commit). Instruction args: `amount: u64, expected_nonce: u64, agent_index: u8` (the index reconstructs the agent PDA seed for the CPI signer, since `AgentWallet` does not store its index)
- [x] 2.2 Implement handler step 1: nonce check + early reject `NonceMismatch`
- [x] 2.3 Implement handler step 2: increment `operator_nonce` (`checked_add`)
- [x] 2.4 Implement handler step 3: invoke `time::needs_*_reset` helpers; zero counters when needed; update `last_*_reset` timestamps
- [x] 2.5 Implement handler steps 4–6: per-tx, daily, hourly checks
- [x] 2.6 Implement handler step 7: whitelist PDA seed + existence verified by Anchor account constraint (handler returns `WhitelistViolation` on missing-account error)
- [x] 2.7 Implement handler step 8: type-1 TTL + `amount_used + amount > approved_amount` checks
- [x] 2.8 Implement handler step 9: call `fee::compute_fee(amount)`
- [x] 2.9 Implement handler step 10: two `token::transfer` CPIs signed with agent PDA seeds (`["wallet", group, idx, bump]`); fee leg skipped when computed fee is zero (sub-cent amounts)
- [x] 2.10 Implement handler step 11: increment `spent_today` (gross) + `tx_count_this_hour` with `checked_add`
- [x] 2.11 Implement handler step 12: type-1 increment `amount_used`; conditional close via `AccountsClose::close` only when exhausted (Anchor's declarative `close = receiver` runs unconditionally, so the trait method is invoked manually with `group_owner` as receiver)
- [x] 2.12 Wire entry point in `lib.rs`

## 3. Tests — happy paths

- [x] 3.1 LiteSVM: transfer to type-0 (intra-group) recipient — covered by `execute_transfer_to_intra_group_recipient_succeeds_with_fee_split` (uses a `PROTOCOL`-typed entry as a stand-in because `add_to_whitelist` rejects `INTRA_GROUP` directly; behavior is identical post-validation)
- [x] 3.2 LiteSVM: transfer to type-1 (external) recipient succeeds; `amount_used` increments — `execute_transfer_to_external_recipient_increments_amount_used`
- [x] 3.3 LiteSVM: transfer to type-2 (protocol) recipient succeeds; no `amount_used` change — `execute_transfer_to_protocol_recipient_does_not_change_amount_used`
- [x] 3.4 LiteSVM: fee math — `amount = 1_000_000` → `net = 999_000`, `fee = 1_000` — `fee_math_one_usdc_yields_999_000_net_and_1_000_fee`
- [x] 3.5 LiteSVM: `spent_today` counts gross, not net — `spent_today_counts_gross_amount`

## 4. Tests — reject paths (one per requirement)

- [x] 4.1 LiteSVM: stale nonce → `NonceMismatch`; no other state mutated — `stale_nonce_rejects_and_leaves_state_unchanged`
- [x] 4.2 LiteSVM: replay (same nonce twice) → second call `NonceMismatch` — `replay_rejects_second_call_with_same_nonce`
- [x] 4.3 LiteSVM: `amount > per_tx_limit` → `PerTxLimitExceeded` — `per_tx_limit_exceeded_rejects`
- [x] 4.4 LiteSVM: `spent_today + amount > daily_limit` → `DailyLimitExceeded` — `daily_limit_exceeded_rejects_after_accumulated_spend`
- [x] 4.5 LiteSVM: `tx_count_this_hour == hourly_tx_cap` → `HourlyCapExceeded` — `hourly_cap_reached_rejects`
- [x] 4.6 LiteSVM: missing whitelist entry → `WhitelistViolation` — `missing_whitelist_entry_rejects`
- [x] 4.7 LiteSVM: type-1 entry past TTL → `WhitelistExpired` — `external_entry_past_ttl_rejects`
- [x] 4.8 LiteSVM: type-1 entry over cap → `WhitelistAmountExhausted` — `external_entry_amount_exhausted_rejects_when_projected_exceeds_cap`
- [x] 4.9 LiteSVM: non-operator signer → `Unauthorized` — `non_operator_signer_rejects_via_has_one`
- [x] 4.9.1 LiteSVM: after `update_backend_operator`, the previously-valid operator's `execute_transfer` fails with `Unauthorized` — `rotated_operator_invalidates_previous_operator`
- [x] 4.10 LiteSVM: zero `amount` → `InvalidAmount` (overflow paths additionally checked via util-level unit tests for `compute_fee`) — `zero_amount_rejects_with_invalid_amount`
- [x] 4.11 LiteSVM: `from_token_account.owner != agent_wallet` → constraint rejection — `from_token_account_owner_mismatch_rejects`
- [x] 4.12 LiteSVM: mint mismatch between `from_token_account` and `to_token_account` → constraint rejection — `mint_mismatch_between_from_and_to_rejects`. (Note: the v1 implementation enforces mint *consistency* across `from`, `to`, and `protocol_fee` rather than pinning to an absolute `USDC_MINT` constant — see design.md §"Mint consistency enforced; absolute USDC pin deferred".)
- [x] 4.13 LiteSVM: `protocol_fee_token_account.owner != group_config.protocol_fee_wallet` → constraint rejection — `protocol_fee_owner_mismatch_rejects`; companion `protocol_fee_account_mint_mismatch_rejects` covers the fee-leg mint mismatch case
- [x] 4.14 LiteSVM: `to_token_account.owner` does not match whitelist PDA seed target → constraint rejection — `whitelist_seed_bound_to_to_token_account_owner`
- [x] 4.15 Property test: for any `amount` in `1..=u64::MAX/2`, `net + fee == amount` (verified by `util::fee::tests::fee_plus_net_always_equals_amount` over a representative sweep including `u64::MAX / 2`)

## 5. Tests — time travel + auto-void

- [x] 5.1 LiteSVM: advance clock past UTC midnight → `spent_today` resets on next call — `daily_counter_resets_after_midnight_crossing`
- [x] 5.2 LiteSVM: advance clock past hour boundary → `tx_count_this_hour` resets on next call — `hourly_counter_resets_after_hour_crossing`
- [x] 5.3 LiteSVM: type-1 transfer that exhausts `approved_amount` → PDA closed, rent returns to owner — `auto_void_closes_pda_when_amount_exhausted_and_returns_rent_to_owner`
- [x] 5.4 LiteSVM: post-auto-void transfer → `WhitelistViolation` (not `WhitelistAmountExhausted`) — `post_auto_void_transfer_fails_with_whitelist_violation`
- [x] 5.5 LiteSVM: after auto-void, orchestrator re-creates whitelist entry via `add_to_whitelist` → succeeds; next transfer succeeds under new cap — `auto_void_and_recreate_works_under_new_cap`

## 6. Integration tests

- [x] 6.1 Mocha: end-to-end against `solana-test-validator` — provision group + agent, fund agent ATA, register external whitelist with $5 cap, execute five $1 transfers totaling $5, assert PDA closed, attempt 6th transfer → fails. Implemented in `tests/execute_transfer.spec.ts` ("end-to-end: external whitelist with $5 cap...") and verified locally via `npm run test:e2e`. The program keypair matching `declare_id!` must be present at `target/deploy/enclz-keypair.json`; that file is gitignored, so cloud sessions need it materialized out-of-band before running e2e.
- [x] 6.2 Mocha: replay protection — submit two txs with same nonce, assert second fails — `tests/execute_transfer.spec.ts` ("nonce replay: ...")

## 7. Verification

- [x] 7.1 `cargo test --package enclz`: 77 tests green (26 lib + 27 execute_transfer + 24 owner_instructions)
- [x] 7.2 `anchor test --validator legacy`: 4 mocha specs green (2 new `execute_transfer` + 2 pre-existing `owner_instructions`)
- [ ] 7.3 Coverage on `execute_transfer.rs` ≥ 90% — every reject branch and the auto-void path are covered by name-mapped LiteSVM tests; explicit coverage tooling (`cargo-llvm-cov`) not yet wired into CI
- [x] 7.4 Manual review: walked steps 1–12 against handler source, line by line
