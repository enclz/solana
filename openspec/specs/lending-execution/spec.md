# lending-execution Specification

## Purpose
Defines requirements for the `execute_lending_op` instruction, which enforces spend policy and protocol fee deduction before routing a deposit or withdrawal through a protocol-whitelisted lending program (e.g., Kamino).

## Requirements

### Requirement: execute_lending_op instruction signature and account constraints

The program SHALL expose `execute_lending_op(op_type: u8, amount: u64, expected_nonce: u64)` where `op_type` is 0 (deposit) or 1 (withdraw), callable only by `GroupConfig.backend_operator`. Required accounts: `backend_operator` (signer), `group_config` (has_one = backend_operator), `agent_wallet` (writable), `agent_token_account` (writable, owner == agent_wallet PDA), `whitelist_entry` (seeds = ["whitelist", group_config, lending_program.key()], entry_type must be 2), `protocol_fee_token_account` (writable, owner == group_config.protocol_fee_wallet, mint == agent_token_account.mint), `lending_program`, `token_program`, `system_program`, plus `remaining_accounts` for lending program-specific accounts.

#### Scenario: Non-operator signer rejected
- **WHEN** any signer other than `GroupConfig.backend_operator` invokes `execute_lending_op`
- **THEN** the call fails with `Unauthorized`

#### Scenario: Non-type-2 lending program rejected
- **WHEN** `whitelist_entry.entry_type != 2` for the supplied lending program
- **THEN** the instruction fails — lending ops are only permitted through protocol-whitelisted programs

#### Scenario: Unknown op_type rejected
- **WHEN** `op_type` is neither 0 nor 1
- **THEN** the call fails with `InvalidAmount`

### Requirement: Nonce check precedes all other validation

#### Scenario: Stale nonce rejected
- **WHEN** `expected_nonce != agent_wallet.operator_nonce`
- **THEN** call fails with `NonceMismatch` and no other state is mutated

### Requirement: Spend-limit enforcement applies to gross amount

Applies to `amount` (gross) before fee deduction, same as `execute_transfer`.

#### Scenario: Daily limit enforced on deposit amount
- **WHEN** `spent_today + amount > daily_limit`
- **THEN** fails with `DailyLimitExceeded`

### Requirement: Deposit — fee deducted from principal before CPI

For `op_type == 0`: compute `protocol_fee = amount * 10 / 10_000`; transfer `protocol_fee` to `protocol_fee_token_account`; CPI into lending program with `net_principal = amount - protocol_fee`.

#### Scenario: Deposit fee math
- **WHEN** `op_type == 0`, `amount = 10_000_000` (10 USDC)
- **THEN** `protocol_fee == 10_000`; lending program receives `9_990_000`

#### Scenario: Deposit CPI failure reverts fee transfer
- **WHEN** lending CPI fails (e.g., pool at capacity)
- **THEN** entire transaction reverts including fee transfer

### Requirement: Withdraw — fee deducted from redeemed amount before crediting agent ATA

For `op_type == 1`: CPI into lending program to redeem, receive `redeemed_amount`; compute `protocol_fee = redeemed_amount * 10 / 10_000`; transfer `protocol_fee` to `protocol_fee_token_account`; net `redeemed_amount - protocol_fee` remains in agent ATA.

#### Scenario: Withdraw fee math
- **WHEN** `op_type == 1` and lending redeems `10_050_000` (principal + yield)
- **THEN** `protocol_fee == 10_050`; agent ATA receives `10_039_950`

#### Scenario: Redeemed amount less than minimum fee rejected
- **WHEN** `redeemed_amount < protocol_fee` (degenerate tiny withdraw)
- **THEN** `checked_sub` fails → `InvalidAmount`; transaction reverts

### Requirement: Counter updates after successful lending op

`spent_today` increments by gross `amount` (input to the instruction); `tx_count_this_hour` increments by 1.

#### Scenario: Hourly cap enforced on lending ops
- **WHEN** `tx_count_this_hour >= hourly_tx_cap`
- **THEN** fails with `HourlyCapExceeded` — lending ops count against the same hourly budget as transfers
