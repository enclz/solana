# swap-execution Specification

## Purpose
Defines requirements for the `execute_swap` instruction, which enforces a custody pin on swap output and protocol fee deduction before routing a token swap through a protocol-whitelisted DEX router (Jupiter v6).

## Requirements

### Requirement: execute_swap instruction signature and account constraints

The program SHALL expose `execute_swap(amount_in: u64, minimum_amount_out: u64, expected_nonce: u64)` callable only by `GroupConfig.backend_operator`. Required accounts: `backend_operator` (signer, also rent payer for `init_if_needed`), `group_config` (has_one = backend_operator), `agent_wallet` (writable), `from_token_account` (writable, **owner == agent_wallet PDA**; mint is unrestricted — any SPL mint the agent holds may be the swap input), `to_token_account` (writable, **owner == agent_wallet PDA** — swap output MUST land in custody of the agent_wallet PDA), `whitelist_entry` (seeds = ["whitelist", group_config, jupiter_program.key()], entry_type must be 2), `protocol_fee_token_account` (writable, owner == group_config.protocol_fee_wallet, mint == `from_token_account.mint`, **created via `init_if_needed`** with `backend_operator` as rent payer), `jupiter_program`, `token_program`, `associated_token_program`, `system_program`, plus `remaining_accounts` for Jupiter route legs. The custody pin on `to_token_account.owner` is the load-bearing safety constraint: the operator may rotate the agent's holdings between any mints, but the proceeds of every swap remain under the agent_wallet PDA's authority.

#### Scenario: Non-operator signer rejected
- **WHEN** any signer other than `GroupConfig.backend_operator` invokes `execute_swap`
- **THEN** the call fails with `Unauthorized`

#### Scenario: Non-type-2 whitelist entry rejected
- **WHEN** caller passes a `whitelist_entry` with `entry_type != 2`
- **THEN** the instruction fails — swap is only permitted through protocol-whitelisted DEX routers

#### Scenario: from_token_account ownership enforced
- **WHEN** `from_token_account.owner != agent_wallet` PDA
- **THEN** Anchor constraint rejects transaction before handler runs

#### Scenario: Free input mint
- **WHEN** caller passes a `from_token_account` whose `mint` is any SPL mint owned by the agent_wallet PDA, including a mint different from `agent_wallet.mint`
- **THEN** the instruction proceeds (no `InvalidMint` rejection on the input leg)

#### Scenario: Swap output must land in agent custody
- **WHEN** caller passes a `to_token_account` whose `owner` is not the agent_wallet PDA
- **THEN** Anchor account constraint rejects the transaction before any swap CPI — the operator cannot redirect swap proceeds to a third-party wallet

#### Scenario: Output mint unrestricted within agent custody
- **WHEN** caller passes a `to_token_account` owned by the agent_wallet PDA whose `mint` differs from both `from_token_account.mint` and `agent_wallet.mint` (the typical swap case)
- **THEN** the constraint check passes; the swap proceeds

#### Scenario: protocol_fee_token_account created on first use of a novel input mint
- **WHEN** caller passes a `from_token_account` of a mint for which no `protocol_fee_token_account` ATA yet exists for `group_config.protocol_fee_wallet`
- **THEN** the instruction creates the fee ATA via `init_if_needed`, charging rent to the `backend_operator` signer, and the swap proceeds in the same transaction

#### Scenario: protocol_fee_token_account misroute rejected
- **WHEN** caller passes a `protocol_fee_token_account` whose `owner` is not `group_config.protocol_fee_wallet`
- **THEN** Anchor account constraint rejects the transaction

### Requirement: Nonce check precedes all other validation

The instruction SHALL verify `expected_nonce == agent_wallet.operator_nonce` and increment the nonce before any other state read, fee transfer, or Jupiter CPI — same precedence as `execute_transfer`.

#### Scenario: Stale nonce rejected
- **WHEN** `expected_nonce != agent_wallet.operator_nonce`
- **THEN** call fails with `NonceMismatch` and no other state is mutated

### Requirement: Rate limiting on swaps

The instruction SHALL enforce `agent_wallet.tx_count_this_hour < hourly_tx_cap` and reset `tx_count_this_hour` when the on-chain clock has crossed the hour boundary since `last_hour_reset`. The instruction SHALL NOT enforce `per_tx_limit`, SHALL NOT enforce `daily_limit`, and SHALL NOT increment `spent_today`. Daily and per-tx limits are denominated in the bound mint's units (e.g., 6-decimal USDC) and are meaningless when applied to an arbitrary swap input mint; they remain in force on `execute_transfer` and `execute_lending_op`. Funds-stay-in-custody removes the theft threat that those limits guarded against on the swap path; `hourly_tx_cap` is retained as a unit-free protection against churn and market-impact abuse.

#### Scenario: Hourly cap reached
- **WHEN** `tx_count_this_hour >= hourly_tx_cap`
- **THEN** the call fails with `HourlyCapExceeded`

#### Scenario: Hourly counter increments on swap
- **WHEN** a swap succeeds
- **THEN** `tx_count_this_hour` increases by exactly 1

#### Scenario: spent_today is not affected by swaps
- **WHEN** a swap of any input amount and any input mint succeeds
- **THEN** `agent_wallet.spent_today` and `agent_wallet.last_spend_reset` are unchanged

#### Scenario: per_tx_limit not enforced
- **WHEN** `amount_in` exceeds the agent's `per_tx_limit` and the `to_token_account` is agent-PDA-owned
- **THEN** the swap proceeds (the per-tx limit applies only to outbound transfers and lending operations)

### Requirement: Fee deduction before Jupiter CPI

The instruction SHALL compute `protocol_fee = amount_in * 10 / 10_000`, transfer `protocol_fee` to `protocol_fee_token_account` via `token::transfer` CPI, then invoke Jupiter v6 with `net_amount_in = amount_in - protocol_fee` and `minimum_amount_out`. The fee is denominated in the input mint.

#### Scenario: Fee deducted before swap
- **WHEN** `amount_in = 1_000_000` of any input mint
- **THEN** `protocol_fee == 1_000` of that input mint is transferred to the fee wallet's ATA for that mint; Jupiter receives `999_000` as input amount

#### Scenario: Swap output below minimum fails
- **WHEN** Jupiter returns fewer tokens than `minimum_amount_out`
- **THEN** Jupiter CPI reverts and the entire transaction reverts including fee transfer and any fee-ATA initialization
