# transfer-execution Specification

## Purpose
execute_transfer instruction — enforced transfer of the bound SPL token from an agent wallet to a whitelisted recipient. Handles nonce-based replay protection, spend-limit checks, whitelist entry validation, and protocol fee deduction.
## Requirements
### Requirement: execute_transfer instruction signature and account constraints

The program SHALL expose `execute_transfer(amount: u64, expected_nonce: u64, agent_index: u8)` callable only by the `backend_operator` recorded on the agent's `GroupConfig`. The `agent_index` parameter reconstructs the `agent_wallet` PDA seed for the SPL token CPI signer; instruction args remain (`amount`, `expected_nonce`) plus this implementation-required index.

Required accounts: `backend_operator` (signer), `group_config`, `group_owner` (writable, address-bound to `group_config.owner`), `agent_wallet` (writable), `from_token_account` (writable, owner == agent_wallet PDA, **mint == `agent_wallet.mint`**), `recipient_wallet` (unchecked, pubkey constrained != protocol_fee_wallet and != agent_wallet PDA), `to_token_account` (writable, init_if_needed via ATA with `recipient_wallet` as authority, **mint == `agent_wallet.mint`**), `whitelist_entry` (seeds derived from `recipient_wallet.key()`), `protocol_fee_token_account` (writable, owner == `group_config.protocol_fee_wallet`, **mint == `agent_wallet.mint`**), `mint` (account matching `agent_wallet.mint`), `token_program`, `associated_token_program`, `system_program`. Mint binding is enforced absolutely against the mint that the agent was provisioned with at `add_agent` time, not via cross-leg parity — the operator cannot select the operating mint at transfer time.

#### Scenario: Non-operator signer rejected
- **WHEN** any signer other than `GroupConfig.backend_operator` invokes `execute_transfer`
- **THEN** the call fails with `Unauthorized`

#### Scenario: from_token_account ownership enforced
- **WHEN** caller passes a `from_token_account` whose `owner` is not the `agent_wallet` PDA
- **THEN** Anchor account constraint rejects the transaction before handler executes

#### Scenario: from_token_account mint must equal agent_wallet.mint
- **WHEN** caller passes a `from_token_account` whose `mint` differs from `agent_wallet.mint` (even if the agent_wallet PDA happens to own an ATA for a different mint)
- **THEN** Anchor account constraint rejects the transaction with `InvalidMint` before any state mutation

#### Scenario: to_token_account mint must equal agent_wallet.mint
- **WHEN** caller passes a `to_token_account` whose `mint != agent_wallet.mint`
- **THEN** Anchor account constraint rejects the transaction with `InvalidMint`

#### Scenario: protocol_fee_token_account mint must equal agent_wallet.mint
- **WHEN** caller passes a `protocol_fee_token_account` whose `mint != agent_wallet.mint`
- **THEN** Anchor account constraint rejects the transaction with `InvalidMint`

#### Scenario: protocol_fee_token_account misroute rejected
- **WHEN** caller passes a `protocol_fee_token_account` whose `owner` is not `group_config.protocol_fee_wallet`
- **THEN** Anchor account constraint rejects the transaction

#### Scenario: to_token_account auto-created if missing
- **WHEN** the recipient does not yet have an ATA for the agent's mint
- **THEN** the `to_token_account` is initialized via `init_if_needed` with `backend_operator` as payer; the transfer proceeds normally

#### Scenario: Recipient wallet equals protocol fee wallet
- **WHEN** `recipient_wallet.key()` equals `group_config.protocol_fee_wallet`
- **THEN** Anchor constraint rejects the transaction with `RecipientInvalid` before the duplicate-mut check fires

#### Scenario: Recipient wallet equals agent PDA
- **WHEN** `recipient_wallet.key()` equals `agent_wallet.key()`
- **THEN** Anchor constraint rejects the transaction with `RecipientInvalid`

#### Scenario: Whitelist seed bound to recipient_wallet
- **WHEN** caller supplies a valid `whitelist_entry` PDA but `recipient_wallet.key()` does not match the PDA's seed target
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

The instruction SHALL require that the supplied `whitelist_entry` PDA matches seeds `["whitelist", group_config, recipient_wallet.key()]` and exists. The seed is derived from `recipient_wallet.key()` (not from `to_token_account.owner`, since the ATA may be uninitialized at resolution time) — so it is impossible to pair a valid whitelist PDA with an unwhitelisted destination. For `entry_type == 1` it SHALL additionally enforce TTL and amount-cap.

#### Scenario: Recipient not whitelisted
- **WHEN** no `WhitelistEntry` PDA exists for the recipient address
- **THEN** Anchor's typed account constraint rejects the transaction with `AccountNotInitialized` (3012) during account resolution, before the handler runs; the backend translates this to `whitelist_violation` for the REST response

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

### Requirement: Whitelist enforcement for EXTERNAL recipients

For `entry_type == 1` (EXTERNAL), the program SHALL enforce TTL expiry and amount-cap: the current clock timestamp MUST be less than or equal to `whitelist_entry.ttl_expires_at`, and `whitelist_entry.amount_used + amount` MUST NOT exceed `whitelist_entry.approved_amount`. The agent's own limits (`daily_limit`, `per_tx_limit`, `hourly_tx_cap`) apply independently on top of the per-recipient cap.

#### Scenario: Transfer succeeds within TTL
- **WHEN** backend operator calls `execute_transfer` to an EXTERNAL recipient with a whitelist entry where `ttl_expires_at >= now`
- **THEN** the transfer succeeds (subject to agent-level limits)

#### Scenario: Transfer fails when TTL expired
- **WHEN** backend operator calls `execute_transfer` to an EXTERNAL recipient with a whitelist entry where `ttl_expires_at < now`
- **THEN** the call fails with `WhitelistExpired`

#### Scenario: Transfer fails when whitelist entry missing
- **WHEN** backend operator calls `execute_transfer` to a recipient address with no `WhitelistEntry` PDA
- **THEN** Anchor account resolution fails (PDA not found), surfacing as `WhitelistViolation`

#### Scenario: Transfer fails when per-recipient cap exhausted
- **WHEN** backend operator calls `execute_transfer` with `amount_used + amount > approved_amount` for an EXTERNAL recipient
- **THEN** the call fails with `WhitelistAmountExhausted`

#### Scenario: Transfer to EXTERNAL recipient succeeds within caps
- **WHEN** backend operator calls `execute_transfer` to an EXTERNAL recipient within TTL and amount-cap
- **THEN** each transfer succeeds, bounded by both per-recipient cap and agent-level limits

### Requirement: Protocol fee deduction

The instruction SHALL compute `protocol_fee = ceil(amount * 10 / 10_000)` using integer ceil arithmetic (`(amount * 10 + 9999) / 10_000`), compute `total = amount + protocol_fee`, transfer `amount` to the recipient ATA (the exact requested amount), and transfer `protocol_fee` to the `protocol_fee_token_account`. Both transfers happen via `token::transfer` CPI signed by the agent wallet PDA. The total drained from the agent's `from_token_account` is `total` (= `amount + protocol_fee`).

#### Scenario: Fee math with standard amount
- **WHEN** `amount = 1_000_000` (1 USDC)
- **THEN** `protocol_fee == 1_000` and `total == 1_001_000`; recipient receives exactly `1_000_000`, fee wallet receives `1_000`

#### Scenario: Fee math with small amount
- **WHEN** `amount = 99`
- **THEN** `protocol_fee == 1` (ceil) and `total == 100`; recipient receives exactly `99`, fee wallet receives `1`

#### Scenario: Fee math with zero amount
- **WHEN** `amount = 0`
- **THEN** the handler rejects with `InvalidAmount` before reaching fee computation

#### Scenario: Fee transfer failure aborts whole instruction
- **WHEN** the fee leg fails (e.g., fee ATA missing)
- **THEN** the entire transaction reverts and the net leg is rolled back

### Requirement: Counter and consumption updates after successful transfer

The instruction SHALL increment `spent_today` by the gross `amount` (not `total`, and not `amount - fee`) and `tx_count_this_hour` by 1. For `entry_type == 1` it SHALL also increment `whitelist_entry.amount_used` by `amount`.

#### Scenario: Spent_today counts request amount
- **WHEN** a transfer of `amount = 1_000_000` succeeds
- **THEN** `spent_today` increases by `1_000_000`, not by `total` (which is `1_001_000`)

#### Scenario: Hourly counter increments
- **WHEN** a transfer succeeds
- **THEN** `tx_count_this_hour` increases by exactly 1

#### Scenario: Amount used incremented for external entry
- **WHEN** a transfer to an `entry_type == 1` recipient succeeds
- **THEN** `whitelist_entry.amount_used` increases by `amount`

### Requirement: Checked arithmetic

All additions and multiplications in the instruction SHALL use `checked_add` / `checked_mul`; overflow SHALL return `InvalidAmount` rather than panic.

#### Scenario: Overflow on amount addition rejected cleanly
- **WHEN** `spent_today + amount` would overflow `u64`
- **THEN** the call fails with `InvalidAmount`, not panic
