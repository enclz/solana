## 1. execute_swap implementation

- [x] 1.1 Create `programs/enclz/src/instructions/execute_swap.rs` with `Accounts` struct: `backend_operator` (signer), `group_config` (has_one = backend_operator), `agent_wallet` (writable), `from_token_account` (mut, owner == agent_wallet, mint == fee mint), `to_token_account` (mut), `whitelist_entry` (seeds-checked, entry_type == 2 asserted in handler), `protocol_fee_token_account` (mut, owner == group_config.protocol_fee_wallet, mint == from mint), `jupiter_program`, `token_program`, `system_program`
- [x] 1.2 Implement handler — steps matching execute_transfer order: nonce → increment → time resets → per_tx / daily / hourly → whitelist type-2 assert → fee calc → fee transfer CPI → Jupiter v6 CPI via remaining_accounts → counter update
- [x] 1.3 Wire `execute_swap` entry point in `lib.rs`
- [x] 1.4 Add constant `JUPITER_V6_PROGRAM_ID: Pubkey` to `constants.rs`

## 2. execute_lending_op implementation

- [x] 2.1 Create `programs/enclz/src/instructions/execute_lending_op.rs` with `Accounts` struct: `backend_operator` (signer), `group_config` (has_one = backend_operator), `agent_wallet` (writable), `agent_token_account` (mut, owner == agent_wallet), `whitelist_entry` (seeds-checked, entry_type == 2 asserted in handler), `protocol_fee_token_account` (mut, owner == group_config.protocol_fee_wallet), `lending_program`, `token_program`, `system_program`
- [x] 2.2 Implement deposit path (op_type == 0): nonce → increment → time resets → limits → whitelist type-2 → fee calc → fee CPI → lending deposit CPI (remaining_accounts) → counter update
- [x] 2.3 Implement withdraw path (op_type == 1): nonce → increment → time resets → limits → whitelist type-2 → lending redeem CPI → fee calc from redeemed → fee CPI → counter update
- [x] 2.4 Reject unknown op_type with `InvalidAmount`
- [x] 2.5 Wire `execute_lending_op` entry point in `lib.rs`

## 3. Tests — execute_swap

- [x] 3.1 LiteSVM: successful swap — fee deducted, Jupiter CPI invoked with net amount, counters increment
- [x] 3.2 LiteSVM: non-type-2 whitelist entry → instruction fails (`WhitelistViolation`)
- [x] 3.3 LiteSVM: stale nonce → `NonceMismatch`
- [x] 3.4 LiteSVM: `amount_in > per_tx_limit` → `PerTxLimitExceeded`
- [x] 3.5 LiteSVM: `spent_today + amount_in > daily_limit` → `DailyLimitExceeded`
- [x] 3.6 LiteSVM: `tx_count_this_hour >= hourly_tx_cap` → `HourlyCapExceeded`
- [x] 3.7 LiteSVM: non-operator signer → `Unauthorized`
- [x] 3.8 LiteSVM: `from_token_account.owner != agent_wallet` → constraint rejection (`InvalidTokenAccount`)
- [x] 3.9 Property test: `fee + net == amount_in` for any `amount_in` (covered exhaustively in `util/fee.rs`; pinned again at the swap-instruction layer in `execute_swap::fee_plus_net_property_pinned_by_lib_unit_test`)

## 4. Tests — execute_lending_op

- [x] 4.1 LiteSVM: successful deposit — fee deducted before CPI, lending program receives net principal, counters increment
- [x] 4.2 LiteSVM: successful withdraw — lending redeems amount (stub mints into agent ATA), fee deducted from redeemed, net lands in agent ATA, counters increment
- [x] 4.3 LiteSVM: non-type-2 whitelist entry → `WhitelistViolation`
- [x] 4.4 LiteSVM: unknown op_type → `InvalidAmount`
- [x] 4.5 LiteSVM: stale nonce → `NonceMismatch`
- [x] 4.6 LiteSVM: daily limit enforced → `DailyLimitExceeded`
- [x] 4.7 LiteSVM: hourly cap enforced → `HourlyCapExceeded`
- [x] 4.8 LiteSVM: zero-redeemed withdraw → `InvalidAmount`. Note: the spec text "redeemed_amount < protocol_fee" is mathematically unreachable (fee = amount × 10/10000 ≤ amount); the realistic failure surface is `redeemed == 0`, which the `require!(redeemed > 0)` check rejects.
- [x] 4.9 LiteSVM: non-operator signer → `Unauthorized`

## 5. Integration tests

- [x] 5.1 Mocha (test-validator + stub program): provision group with stub-as-Jupiter type-2 whitelist, fund agent ATA, execute swap, assert fee transferred and `spent_today` incremented by gross input amount. Stub stands in for Jupiter v6; live devnet Jupiter integration is deferred to the staging deploy in `add-devnet-deploy-pipeline`.
- [x] 5.2 Mocha: deposit through stub-as-Kamino, assert protocol fee received and counters bumped. Live Kamino devnet is deferred to the staging deploy.
- [x] 5.3 Mocha: withdraw through stub-as-Kamino — stub mints `redeemed` to agent ATA, fee is deducted from the delta, net 999_000 lands in agent ATA. Live Kamino devnet redeem is deferred to the staging deploy.

## 6. Verification

- [x] 6.1 `cargo test --package enclz`: all unit + LiteSVM tests green (28 lib + 9 swap + 9 lending + 26 transfer + 24 owner = 96 tests passing)
- [x] 6.2 `npm run test:e2e` (anchor + Mocha): 7 integration tests passing against test-validator with the stub program loaded via `[[test.genesis]]`
- [x] 6.3 Manual review: confirmed fee-before-swap ordering at `execute_swap.rs` Step 10 (fee CPI) precedes Step 11 (Jupiter CPI); confirmed type-2 whitelist check cannot be bypassed because the whitelist PDA seed binds `["whitelist", group, jupiter_program]` and the handler asserts `entry_type == PROTOCOL` before any CPI runs
