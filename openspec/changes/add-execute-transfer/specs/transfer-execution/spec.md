## ADDED Requirements

### Requirement: execute_transfer instruction signature and account constraints

The program SHALL expose `execute_transfer(amount: u64, expected_nonce: u64, agent_index: u8)` callable only by the `backend_operator` recorded on the agent's `GroupConfig`. The `agent_index` parameter reconstructs the `agent_wallet` PDA seed for the SPL token CPI signer; instruction args remain (`amount`, `expected_nonce`) plus this implementation-required index.

Required accounts: `backend_operator` (signer), `group_config`, `group_owner` (writable, address-bound to `group_config.owner` — receives auto-void rent), `agent_wallet` (writable), `from_token_account` (writable, owner == agent_wallet PDA, mint matches `to_token_account.mint` and `protocol_fee_token_account.mint`), `to_token_account` (writable), `whitelist_entry` (seeds derived from `to_token_account.owner`), `protocol_fee_token_account` (writable, owner == `group_config.protocol_fee_wallet`), `token_program`, `system_program`.

#### Scenario: Non-operator signer rejected
- **WHEN** any signer other than `GroupConfig.backend_operator` invokes `execute_transfer`
- **THEN** the call fails with `Unauthorized`

#### Scenario: from_token_account ownership enforced
- **WHEN** caller passes a `from_token_account` whose `owner` is not the `agent_wallet` PDA
- **THEN** Anchor account constraint rejects the transaction before handler executes

#### Scenario: Mint mismatch across token accounts rejected
- **WHEN** `from_token_account.mint`, `to_token_account.mint`, and `protocol_fee_token_account.mint` are not all equal
- **THEN** Anchor account constraint rejects the transaction. (Mint consistency is the v1 enforcement: the orchestrator's choice at agent creation determines the operating mint — typically USDC — and the program ensures all three legs of a transfer use the same mint, eliminating cross-mint exploits.)

#### Scenario: protocol_fee_token_account misroute rejected
- **WHEN** caller passes a `protocol_fee_token_account` whose `owner` is not `group_config.protocol_fee_wallet`
- **THEN** Anchor account constraint rejects the transaction

#### Scenario: Whitelist seed bound to to_token_account.owner
- **WHEN** caller supplies a valid `whitelist_entry` PDA but `to_token_account.owner` does not match the PDA's seed target
- **THEN** Anchor seed constraint rejects the transaction — no whitelist bypass possible via account substitution

### Requirement: Nonce check precedes all other validation

The instruction SHALL verify `expected_nonce == agent_wallet.operator_nonce` and increment the nonce before any whitelist lookup, limit check, or token transfer.

#### Scenario: Stale nonce rejected
- **WHEN** caller passes `expected_nonce` not equal to the current `operator_nonce`
- **THEN** the call fails with `NonceMismatch` and no other state is mutated

#### Scenario: Successful call increments nonce
- **WHEN** a valid `execute_transfer` succeeds with `expected_nonce == N`
- **THEN** subsequent reads show `operator_nonce == N + 1`

#### Scenario: Replay rejected
- **WHEN** the same `expected_nonce` is submitted twice
- **THEN** the second call fails with `NonceMismatch`

### Requirement: Daily and hourly counter resets

The instruction SHALL reset `spent_today` to zero when `Clock::get().unix_timestamp` has crossed UTC midnight since `last_spend_reset`, and reset `tx_count_this_hour` when the hour boundary has passed since `last_hour_reset`.

#### Scenario: Daily reset on midnight crossing
- **WHEN** the on-chain clock advances past UTC midnight after a previous transfer left `spent_today > 0`
- **THEN** the next `execute_transfer` recomputes `spent_today` from zero before applying the daily-limit check

#### Scenario: Hourly reset on hour crossing
- **WHEN** the on-chain clock advances past the next hour after a previous transfer left `tx_count_this_hour > 0`
- **THEN** the next `execute_transfer` recomputes `tx_count_this_hour` from zero before applying the hourly-cap check

### Requirement: Spend-limit enforcement

The instruction SHALL reject the transfer with the spec-mandated error variant when:
- `amount > per_tx_limit` → `PerTxLimitExceeded`
- `spent_today + amount > daily_limit` → `DailyLimitExceeded`
- `tx_count_this_hour >= hourly_tx_cap` → `HourlyCapExceeded`

#### Scenario: Per-tx limit exceeded
- **WHEN** `amount` exceeds `per_tx_limit`
- **THEN** the call fails with `PerTxLimitExceeded` and no token transfer occurs

#### Scenario: Daily limit exceeded
- **WHEN** `spent_today + amount > daily_limit`
- **THEN** the call fails with `DailyLimitExceeded`

#### Scenario: Hourly cap reached
- **WHEN** `tx_count_this_hour == hourly_tx_cap`
- **THEN** the call fails with `HourlyCapExceeded`

### Requirement: Whitelist enforcement

The instruction SHALL require that the supplied `whitelist_entry` PDA matches seeds `["whitelist", group_config, to_token_account.owner]` and exists. The seed is derived from `to_token_account.owner` — not an independent `recipient` argument — so it is impossible to pair a valid whitelist PDA with an unwhitelisted destination ATA. For `entry_type == 1` it SHALL additionally enforce TTL and amount-cap.

#### Scenario: Recipient not whitelisted
- **WHEN** no `WhitelistEntry` PDA exists for the recipient address
- **THEN** the call fails with `WhitelistViolation`

#### Scenario: External entry expired
- **WHEN** `entry_type == 1` and `now > ttl_expires_at`
- **THEN** the call fails with `WhitelistExpired`

#### Scenario: External entry amount exhausted
- **WHEN** `entry_type == 1` and `amount_used + amount > approved_amount`
- **THEN** the call fails with `WhitelistAmountExhausted`

#### Scenario: Intra-group transfer always allowed within spend limits
- **WHEN** `entry_type == 0` and all spend-limit checks pass
- **THEN** the transfer succeeds regardless of TTL or amount fields

#### Scenario: Protocol entry always allowed within spend limits
- **WHEN** `entry_type == 2` and all spend-limit checks pass
- **THEN** the transfer succeeds regardless of TTL or amount fields

### Requirement: Protocol fee deduction

The instruction SHALL compute `protocol_fee = amount * 10 / 10_000`, transfer `net = amount - fee` to the recipient ATA, and transfer `protocol_fee` to the `protocol_fee_token_account`. Both transfers happen via `token::transfer` CPI signed by the agent wallet PDA.

#### Scenario: Fee math
- **WHEN** `amount = 1_000_000` (1 USDC)
- **THEN** `protocol_fee == 1_000` and `net == 999_000`; both ATAs reflect the transfers

#### Scenario: Fee transfer failure aborts whole instruction
- **WHEN** the fee leg fails (e.g., fee ATA missing)
- **THEN** the entire transaction reverts and the net leg is rolled back

### Requirement: Counter and consumption updates after successful transfer

The instruction SHALL increment `spent_today` by the gross `amount` (not `net`) and `tx_count_this_hour` by 1. For `entry_type == 1` it SHALL also increment `whitelist_entry.amount_used` by `amount`.

#### Scenario: Spent_today counts gross
- **WHEN** a transfer of `amount = 1_000_000` succeeds
- **THEN** `spent_today` increases by `1_000_000`, not `999_000`

#### Scenario: Hourly counter increments
- **WHEN** a transfer succeeds
- **THEN** `tx_count_this_hour` increases by exactly 1

#### Scenario: Amount used incremented for external entry
- **WHEN** a transfer to an `entry_type == 1` recipient succeeds
- **THEN** `whitelist_entry.amount_used` increases by `amount`

### Requirement: Auto-void on whitelist exhaustion

When a successful transfer causes `entry_type == 1` `amount_used >= approved_amount`, the instruction SHALL close the `WhitelistEntry` PDA and return rent lamports to the group owner.

#### Scenario: Exact exhaustion closes entry
- **WHEN** a transfer brings `amount_used` to exactly `approved_amount`
- **THEN** the `WhitelistEntry` PDA is closed and lamports return to the orchestrator

#### Scenario: Subsequent transfer fails as whitelist_violation, not amount_exhausted
- **WHEN** an agent attempts another transfer to the same address after auto-void
- **THEN** the call fails with `WhitelistViolation` (not `WhitelistAmountExhausted`) because the PDA no longer exists

### Requirement: Checked arithmetic

All additions and multiplications in the instruction SHALL use `checked_add` / `checked_mul`; overflow SHALL return `InvalidAmount` rather than panic.

#### Scenario: Overflow on amount addition rejected cleanly
- **WHEN** `spent_today + amount` would overflow `u64`
- **THEN** the call fails with `InvalidAmount`, not panic
