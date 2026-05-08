## MODIFIED Requirements

### Requirement: execute_lending_op instruction signature and account constraints

The program SHALL expose `execute_lending_op(op_type: u8, amount: u64, expected_nonce: u64)` where `op_type` is 0 (deposit) or 1 (withdraw), callable only by `GroupConfig.backend_operator`. Required accounts: `backend_operator` (signer), `group_config` (has_one = backend_operator), `agent_wallet` (writable), `agent_token_account` (writable, owner == agent_wallet PDA, **mint == `agent_wallet.mint`**), `whitelist_entry` (seeds = ["whitelist", group_config, lending_program.key()], entry_type must be 2), `protocol_fee_token_account` (writable, owner == group_config.protocol_fee_wallet, **mint == `agent_wallet.mint`**), `lending_program`, `token_program`, `system_program`, plus `remaining_accounts` for lending program-specific accounts. The principal and fee mints are pinned absolutely to the agent's bound mint; cross-leg parity is no longer the enforcement mechanism.

#### Scenario: Non-operator signer rejected
- **WHEN** any signer other than `GroupConfig.backend_operator` invokes `execute_lending_op`
- **THEN** the call fails with `Unauthorized`

#### Scenario: Non-type-2 lending program rejected
- **WHEN** `whitelist_entry.entry_type != 2` for the supplied lending program
- **THEN** the instruction fails — lending ops are only permitted through protocol-whitelisted programs

#### Scenario: Unknown op_type rejected
- **WHEN** `op_type` is neither 0 nor 1
- **THEN** the call fails with `InvalidAmount`

#### Scenario: agent_token_account mint must equal agent_wallet.mint
- **WHEN** caller passes an `agent_token_account` whose `mint != agent_wallet.mint`
- **THEN** Anchor account constraint rejects the transaction with `InvalidMint` before any lending CPI

#### Scenario: protocol_fee_token_account mint must equal agent_wallet.mint
- **WHEN** caller passes a `protocol_fee_token_account` whose `mint != agent_wallet.mint`
- **THEN** Anchor account constraint rejects the transaction with `InvalidMint`
