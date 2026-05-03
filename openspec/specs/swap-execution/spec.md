# swap-execution Specification

## Purpose
Defines requirements for the `execute_swap` instruction, which enforces spend policy and protocol fee deduction before routing a token swap through a protocol-whitelisted DEX router (Jupiter v6).

## Requirements

### Requirement: execute_swap instruction signature and account constraints

The program SHALL expose `execute_swap(amount_in: u64, minimum_amount_out: u64, expected_nonce: u64)` callable only by `GroupConfig.backend_operator`. Required accounts: `backend_operator` (signer), `group_config` (has_one = backend_operator), `agent_wallet` (writable), `from_token_account` (writable, owner == agent_wallet PDA, mint == USDC), `to_token_account` (writable), `whitelist_entry` (seeds = ["whitelist", group_config, jupiter_program.key()], entry_type must be 2), `protocol_fee_token_account` (writable, owner == group_config.protocol_fee_wallet, mint == USDC), `jupiter_program`, `token_program`, `system_program`, plus `remaining_accounts` for Jupiter route legs.

#### Scenario: Non-operator signer rejected
- **WHEN** any signer other than `GroupConfig.backend_operator` invokes `execute_swap`
- **THEN** the call fails with `Unauthorized`

#### Scenario: Non-type-2 whitelist entry rejected
- **WHEN** caller passes a `whitelist_entry` with `entry_type != 2`
- **THEN** the instruction fails — swap is only permitted through protocol-whitelisted DEX routers

#### Scenario: from_token_account ownership enforced
- **WHEN** `from_token_account.owner != agent_wallet` PDA
- **THEN** Anchor constraint rejects transaction before handler runs

### Requirement: Nonce check precedes all other validation

Same as `execute_transfer`: nonce checked and incremented before any other state read or mutation.

#### Scenario: Stale nonce rejected
- **WHEN** `expected_nonce != agent_wallet.operator_nonce`
- **THEN** call fails with `NonceMismatch` and no other state is mutated

### Requirement: Spend-limit enforcement on amount_in

The instruction SHALL apply per-tx, daily, and hourly checks to `amount_in` (gross, before fee deduction) using the same logic as `execute_transfer`.

#### Scenario: Per-tx limit exceeded
- **WHEN** `amount_in > per_tx_limit`
- **THEN** fails with `PerTxLimitExceeded`

#### Scenario: Daily limit exceeded
- **WHEN** `spent_today + amount_in > daily_limit`
- **THEN** fails with `DailyLimitExceeded`

#### Scenario: Hourly cap reached
- **WHEN** `tx_count_this_hour >= hourly_tx_cap`
- **THEN** fails with `HourlyCapExceeded`

### Requirement: Fee deduction before Jupiter CPI

The instruction SHALL compute `protocol_fee = amount_in * 10 / 10_000`, transfer `protocol_fee` to `protocol_fee_token_account` via `token::transfer` CPI, then invoke Jupiter v6 with `net_amount_in = amount_in - protocol_fee` and `minimum_amount_out`.

#### Scenario: Fee deducted before swap
- **WHEN** `amount_in = 1_000_000`
- **THEN** `protocol_fee == 1_000` is transferred to fee wallet; Jupiter receives `999_000` as input amount

#### Scenario: Swap output below minimum fails
- **WHEN** Jupiter returns fewer tokens than `minimum_amount_out`
- **THEN** Jupiter CPI reverts and the entire transaction reverts including fee transfer

### Requirement: Counter updates after successful swap

`spent_today` increments by gross `amount_in`; `tx_count_this_hour` increments by 1. No `amount_used` update (whitelist entry is type-2 — permanent, uncapped).

#### Scenario: Spent_today counts gross amount_in
- **WHEN** swap of `amount_in = 1_000_000` succeeds
- **THEN** `spent_today` increases by `1_000_000`
