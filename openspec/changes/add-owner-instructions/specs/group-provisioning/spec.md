## ADDED Requirements

### Requirement: initialize_group instruction

The program SHALL expose `initialize_group(backend_operator: Pubkey, protocol_fee_wallet: Pubkey, dex_router: Pubkey)` that creates a `GroupConfig` PDA owned by the signer and atomically creates a type-2 `WhitelistEntry` for `dex_router`.

#### Scenario: Successful group creation
- **WHEN** signer calls `initialize_group` with valid pubkeys
- **THEN** a `GroupConfig` PDA exists with `owner == signer`, `backend_operator` + `protocol_fee_wallet` set, `agent_count == 0`, and a type-2 `WhitelistEntry` PDA exists for `dex_router`

#### Scenario: DEX router auto-whitelisted at init
- **WHEN** signer calls `initialize_group` with `dex_router = JUPITER_PROGRAM_ID`
- **THEN** a `WhitelistEntry` PDA with seeds `["whitelist", group, JUPITER_PROGRAM_ID]` and `entry_type == 2` is created atomically in the same transaction

#### Scenario: Duplicate group rejected
- **WHEN** the same signer calls `initialize_group` twice
- **THEN** the second call fails because the PDA already exists

### Requirement: add_agent instruction

The program SHALL expose `add_agent(display_name: [u8;32], daily_limit: Option<u64>, per_tx_limit: Option<u64>, hourly_tx_cap: Option<u8>)` callable only by the group owner. It creates an `AgentWallet` PDA, creates the agent's USDC ATA via CPI, auto-creates an intra-group `WhitelistEntry` (entry_type=0, ttl=0, amount=0) for the new agent's pubkey, and increments `GroupConfig.agent_count`.

#### Scenario: Successful add with defaults
- **WHEN** owner calls `add_agent` with all `Option` args `None`
- **THEN** the agent's `daily_limit == 10_000_000`, `per_tx_limit == 1_000_000`, `hourly_tx_cap == 5`, ATA is created, intra-group whitelist entry exists, and `agent_count` increments by 1

#### Scenario: Override applied
- **WHEN** owner calls `add_agent` with `daily_limit: Some(50_000_000)`
- **THEN** stored `daily_limit == 50_000_000`; other limits use defaults

#### Scenario: Non-owner rejected
- **WHEN** any non-owner signer calls `add_agent`
- **THEN** the call fails with `Unauthorized` before handler executes

### Requirement: update_agent_limits instruction

The program SHALL expose `update_agent_limits(daily_limit: Option<u64>, per_tx_limit: Option<u64>, hourly_tx_cap: Option<u8>)` callable only by the group owner; `Some` values overwrite the field, `None` leaves it unchanged.

#### Scenario: Patch a single field
- **WHEN** owner calls with `daily_limit: Some(5_000_000)`, others `None`
- **THEN** only `daily_limit` changes; `per_tx_limit` and `hourly_tx_cap` retain prior values

### Requirement: update_backend_operator instruction

The program SHALL expose `update_backend_operator(new_operator: Pubkey)` callable only by the group owner; replaces `GroupConfig.backend_operator`.

#### Scenario: Operator rotation
- **WHEN** owner calls `update_backend_operator` with a new pubkey
- **THEN** subsequent `execute_transfer` calls signed by the old operator fail and ones signed by the new operator succeed

### Requirement: emergency_withdraw instruction

The program SHALL expose `emergency_withdraw(destination: Pubkey)` callable only by the group owner. It bypasses spend limits and operator nonce, transferring the full agent ATA balance via SPL token CPI to the destination ATA.

#### Scenario: Sweep all funds
- **WHEN** owner calls `emergency_withdraw` against an agent ATA holding any positive balance
- **THEN** all tokens are transferred to the destination ATA and the agent ATA balance is zero

#### Scenario: Non-owner rejected
- **WHEN** any non-owner signer (including the backend operator) calls `emergency_withdraw`
- **THEN** the call fails with `Unauthorized`
