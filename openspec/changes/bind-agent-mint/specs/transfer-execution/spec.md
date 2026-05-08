## MODIFIED Requirements

### Requirement: execute_transfer instruction signature and account constraints

The program SHALL expose `execute_transfer(amount: u64, expected_nonce: u64, agent_index: u8)` callable only by the `backend_operator` recorded on the agent's `GroupConfig`. The `agent_index` parameter reconstructs the `agent_wallet` PDA seed for the SPL token CPI signer; instruction args remain (`amount`, `expected_nonce`) plus this implementation-required index.

Required accounts: `backend_operator` (signer), `group_config`, `group_owner` (writable, address-bound to `group_config.owner` — receives auto-void rent), `agent_wallet` (writable), `from_token_account` (writable, owner == agent_wallet PDA, **mint == `agent_wallet.mint`**), `to_token_account` (writable, **mint == `agent_wallet.mint`**), `whitelist_entry` (seeds derived from `to_token_account.owner`), `protocol_fee_token_account` (writable, owner == `group_config.protocol_fee_wallet`, **mint == `agent_wallet.mint`**), `token_program`, `system_program`. Mint binding is enforced absolutely against the mint that the agent was provisioned with at `add_agent` time, not via cross-leg parity — the operator cannot select the operating mint at transfer time.

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

#### Scenario: Whitelist seed bound to to_token_account.owner
- **WHEN** caller supplies a valid `whitelist_entry` PDA but `to_token_account.owner` does not match the PDA's seed target
- **THEN** Anchor seed constraint rejects the transaction — no whitelist bypass possible via account substitution
