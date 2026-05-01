## 1. execute_swap implementation

- [ ] 1.1 Create `programs/enclz/src/instructions/execute_swap.rs` with `Accounts` struct: `backend_operator` (signer), `group_config` (has_one = backend_operator), `agent_wallet` (writable), `from_token_account` (mut, owner == agent_wallet, mint == USDC), `to_token_account` (mut), `whitelist_entry` (seeds-checked, entry_type == 2 asserted in handler), `protocol_fee_token_account` (mut, owner == group_config.protocol_fee_wallet, mint == USDC), `jupiter_program`, `token_program`, `system_program`
- [ ] 1.2 Implement handler — steps matching execute_transfer order: nonce → increment → time resets → per_tx / daily / hourly → whitelist type-2 assert → fee calc → fee transfer CPI → Jupiter v6 CPI via remaining_accounts → counter update
- [ ] 1.3 Wire `execute_swap` entry point in `lib.rs`
- [ ] 1.4 Add constant `JUPITER_V6_PROGRAM_ID: Pubkey` to `constants.rs`

## 2. execute_lending_op implementation

- [ ] 2.1 Create `programs/enclz/src/instructions/execute_lending_op.rs` with `Accounts` struct: `backend_operator` (signer), `group_config` (has_one = backend_operator), `agent_wallet` (writable), `agent_token_account` (mut, owner == agent_wallet), `whitelist_entry` (seeds-checked, entry_type == 2 asserted in handler), `protocol_fee_token_account` (mut, owner == group_config.protocol_fee_wallet), `lending_program`, `token_program`, `system_program`
- [ ] 2.2 Implement deposit path (op_type == 0): nonce → increment → time resets → limits → whitelist type-2 → fee calc → fee CPI → lending deposit CPI (remaining_accounts) → counter update
- [ ] 2.3 Implement withdraw path (op_type == 1): nonce → increment → time resets → limits → whitelist type-2 → lending redeem CPI → fee calc from redeemed → fee CPI → counter update
- [ ] 2.4 Reject unknown op_type with `InvalidAmount`
- [ ] 2.5 Wire `execute_lending_op` entry point in `lib.rs`

## 3. Tests — execute_swap

- [ ] 3.1 LiteSVM: successful swap — fee deducted, Jupiter CPI invoked with net amount, counters increment
- [ ] 3.2 LiteSVM: non-type-2 whitelist entry → instruction fails
- [ ] 3.3 LiteSVM: stale nonce → `NonceMismatch`
- [ ] 3.4 LiteSVM: `amount_in > per_tx_limit` → `PerTxLimitExceeded`
- [ ] 3.5 LiteSVM: `spent_today + amount_in > daily_limit` → `DailyLimitExceeded`
- [ ] 3.6 LiteSVM: `tx_count_this_hour >= hourly_tx_cap` → `HourlyCapExceeded`
- [ ] 3.7 LiteSVM: non-operator signer → `Unauthorized`
- [ ] 3.8 LiteSVM: `from_token_account.owner != agent_wallet` → constraint rejection
- [ ] 3.9 Property test: `fee + net == amount_in` for any `amount_in`

## 4. Tests — execute_lending_op

- [ ] 4.1 LiteSVM: successful deposit — fee deducted before CPI, lending program receives net principal, counters increment
- [ ] 4.2 LiteSVM: successful withdraw — lending redeems amount, fee deducted from redeemed, net lands in agent ATA, counters increment
- [ ] 4.3 LiteSVM: non-type-2 whitelist entry → instruction fails
- [ ] 4.4 LiteSVM: unknown op_type → `InvalidAmount`
- [ ] 4.5 LiteSVM: stale nonce → `NonceMismatch`
- [ ] 4.6 LiteSVM: daily limit enforced → `DailyLimitExceeded`
- [ ] 4.7 LiteSVM: hourly cap enforced → `HourlyCapExceeded`
- [ ] 4.8 LiteSVM: redeemed amount less than fee → `InvalidAmount`
- [ ] 4.9 LiteSVM: non-operator signer → `Unauthorized`

## 5. Integration tests

- [ ] 5.1 Mocha (test-validator + devnet Jupiter fork): provision group with Jupiter as type-2 whitelist, fund agent ATA, execute swap, assert output token arrives, assert `spent_today` incremented by gross input amount
- [ ] 5.2 Mocha: deposit into Kamino devnet pool, assert protocol fee received, assert lending receipt token in agent ATA
- [ ] 5.3 Mocha: withdraw from Kamino devnet pool, assert fee deducted from redeemed, assert net in agent ATA

## 6. Verification

- [ ] 6.1 `cargo test --package enclz`: all unit tests green
- [ ] 6.2 `anchor test`: integration tests green
- [ ] 6.3 Coverage on `execute_swap.rs` and `execute_lending_op.rs` ≥ 85%
- [ ] 6.4 Manual review: confirm fee-before-swap ordering; confirm type-2 whitelist check cannot be bypassed
- [ ] 6.5 Backend confirms `/v1/swap`, `/v1/deposit`, `/v1/withdraw` integrate against deployed program
