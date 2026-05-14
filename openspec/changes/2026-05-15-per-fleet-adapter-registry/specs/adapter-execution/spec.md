## ADDED Requirements

### Requirement: execute_via_adapter instruction signature and account constraints

The program SHALL expose `execute_via_adapter(adapter_id: Pubkey, call_data: Vec<u8>, amount: u64, expected_nonce: u64, agent_index: u8)` callable only by the `backend_operator` recorded on the agent's `GroupConfig`. The instruction validates the adapter is owner-approved, applies the same spend-limit + frequency policy checks as `execute_transfer`, and CPIs into the adapter program with the agent_wallet PDA's signer seeds.

Required accounts: `backend_operator` (signer), `group_config`, `agent_wallet` (writable), `adapter_registry`, `adapter_program` (unchecked program account whose `key()` must equal `adapter_id` and must match an `Active` entry in `adapter_registry.entries`), plus `..remaining_accounts` forwarded to the adapter. The instruction enforces that every writable token account in `remaining_accounts` whose owner is the agent_wallet PDA is being mutated by the adapter only in ways consistent with the agent's custody (post-CPI ownership check).

The instruction SHALL:
- Reject if the signer is not `group_config.backend_operator` with `Unauthorized`
- Reject if `adapter_id` is not present in `adapter_registry.entries` with `status == Active` (`AdapterNotApproved`)
- Reject if `adapter_program.key() != adapter_id` with `AdapterNotApproved`
- Reject if `adapter_id` is the SPL Token program, Associated Token program, or any BPF loader program ID with `AdapterNotApproved` (defense-in-depth)
- Apply the agent's `per_tx_limit`, `daily_limit`, and `hourly_tx_cap` policy checks against `amount`
- Increment `agent_wallet.spent_today` and the frequency counter on success
- Validate `expected_nonce` matches `agent_wallet.nonce`; advance `nonce += 1` on success (replay protection identical to `execute_transfer`)
- Invoke the adapter via `invoke_signed` passing `call_data` and the entry's stored `constraints: Vec<u8>` as adapter input, with the agent_wallet PDA seeds as signer
- After the CPI returns, verify that every writable account in `remaining_accounts` whose `owner == agent_wallet PDA` is still owned by the agent_wallet PDA (custody post-check)

#### Scenario: Non-operator signer rejected
- **WHEN** any signer other than `GroupConfig.backend_operator` invokes `execute_via_adapter`
- **THEN** the call fails with `Unauthorized`

#### Scenario: Unregistered adapter rejected
- **WHEN** `adapter_id` does not appear in `adapter_registry.entries`
- **THEN** the call fails with `AdapterNotApproved` before any CPI

#### Scenario: Paused adapter rejected
- **WHEN** `adapter_id` appears in `adapter_registry.entries` but its `status == Paused`
- **THEN** the call fails with `AdapterNotApproved`

#### Scenario: adapter_program account mismatched with adapter_id rejected
- **WHEN** the passed `adapter_program.key()` differs from `adapter_id`
- **THEN** the call fails with `AdapterNotApproved`

#### Scenario: Forbidden CPI target rejected
- **WHEN** `adapter_id` equals the SPL Token program ID, the Associated Token program ID, or any BPF loader program ID
- **THEN** the call fails with `AdapterNotApproved`

#### Scenario: Per-tx limit enforced
- **WHEN** `amount > agent_wallet.per_tx_limit`
- **THEN** the call fails with `PerTxLimitExceeded` before any CPI

#### Scenario: Daily limit enforced
- **WHEN** `agent_wallet.spent_today + amount > agent_wallet.daily_limit`
- **THEN** the call fails with `DailyLimitExceeded` before any CPI

#### Scenario: Frequency cap enforced
- **WHEN** the hourly transaction count would exceed `agent_wallet.hourly_tx_cap`
- **THEN** the call fails with `FrequencyCapExceeded` before any CPI

#### Scenario: Nonce mismatch rejected
- **WHEN** `expected_nonce != agent_wallet.nonce`
- **THEN** the call fails with `NonceMismatch`

#### Scenario: Custody post-check on writable PDA-owned token accounts
- **WHEN** the adapter's CPI changes the owner of a writable token account that was previously owned by the agent_wallet PDA
- **THEN** the transaction reverts after the CPI returns with `CustodyViolation`

#### Scenario: constraints passed to adapter verbatim
- **WHEN** the registry entry for `adapter_id` has `constraints: Vec<u8>` of arbitrary length
- **THEN** those exact bytes are forwarded to the adapter's entry-point as the constraints input

#### Scenario: Nonce advances on success
- **WHEN** `execute_via_adapter` completes successfully
- **THEN** `agent_wallet.nonce` is incremented by exactly 1
