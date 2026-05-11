# program-state Specification

## Purpose
TBD - created by archiving change init-anchor-workspace. Update Purpose after archive.
## Requirements
### Requirement: GroupConfig PDA

The program SHALL define a `GroupConfig` account derived from seeds `["group", owner_pubkey]` containing the orchestrator's owner pubkey, the backend operator pubkey, the protocol fee wallet pubkey, an `agent_count: u8`, and a `group_name: [u8; 32]` storing a fixed-width human-readable label written verbatim with no on-chain encoding validation.

#### Scenario: PDA derivation matches spec
- **WHEN** test derives a PDA using seeds `[b"group", owner.key().as_ref()]` for any test owner pubkey
- **THEN** the resulting PDA equals `Pubkey::find_program_address` output and matches what the program produces internally

#### Scenario: Account size accommodates all fields
- **WHEN** test allocates a `GroupConfig` account using `8 + GroupConfig::INIT_SPACE`
- **THEN** all five fields can be written and read back without panic, and `INIT_SPACE` equals `32 + 32 + 32 + 1 + 32`

#### Scenario: group_name round-trips unchanged
- **WHEN** test serializes a `GroupConfig` whose `group_name` is any non-zero 32-byte pattern (including bytes that are not valid UTF-8) and then deserializes the buffer
- **THEN** the decoded `group_name` is byte-for-byte equal to the input

### Requirement: AgentWallet PDA

The program SHALL define an `AgentWallet` account derived from seeds `["wallet", group_pubkey, agent_index]` containing the group pubkey, the SPL token mint the agent is bound to (set once at `add_agent` and immutable thereafter), a fixed 32-byte display name, daily/per-tx/hourly limits in token-native 6-decimal units, current-period counters, last-reset timestamps, a `u64 operator_nonce`, and a `bump: u8` storing the canonical PDA bump.

#### Scenario: PDA derivation matches spec
- **WHEN** test derives `[b"wallet", group.as_ref(), &[idx]]` for any group + index
- **THEN** result matches the program-side PDA bump

#### Scenario: Default limits applied at init
- **WHEN** an `AgentWallet` is initialized with `daily_limit: None`, `per_tx_limit: None`, `hourly_tx_cap: None`
- **THEN** values default to 10_000_000, 1_000_000, and 5 respectively

#### Scenario: Mint round-trips unchanged
- **WHEN** test serializes an `AgentWallet` populated with an arbitrary `mint: Pubkey` and deserializes the buffer
- **THEN** the decoded `mint` is byte-for-byte equal to the input

#### Scenario: INIT_SPACE accommodates all fields including mint
- **WHEN** test reads `AgentWallet::INIT_SPACE`
- **THEN** the value equals `32 (group) + 32 (mint) + 32 (display_name) + 8 + 8 + 1 + 8 + 1 + 8 + 8 + 8 + 1 (bump) = 147`

### Requirement: WhitelistEntry PDA

The program SHALL define a `WhitelistEntry` account derived from seeds `["whitelist", group_pubkey, target_address]` containing `label: [u8; 32]`, `target: Pubkey`, `added_by: Pubkey`, `entry_type: u8` (0=intra, 1=external, 2=protocol), `ttl_expires_at: i64`, and a `bump: u8` storing the canonical PDA bump.

#### Scenario: PDA derivation matches spec
- **WHEN** test derives `[b"whitelist", group.as_ref(), target.as_ref()]`
- **THEN** result matches program-side PDA

#### Scenario: Entry type values match documented enum
- **WHEN** test reads the `entry_type` constants
- **THEN** `INTRA_GROUP == 0`, `EXTERNAL == 1`, `PROTOCOL == 2`

#### Scenario: INIT_SPACE accommodates remaining fields
- **WHEN** test reads `WhitelistEntry::INIT_SPACE`
- **THEN** the value equals `32 (label) + 32 (target) + 32 (added_by) + 1 (entry_type) + 8 (ttl_expires_at) + 1 (bump) = 106`

### Requirement: Error enum mirrors backend codes

The program SHALL define error variants whose names map 1:1 to backend REST error codes: `WhitelistViolation`, `WhitelistExpired`, `WhitelistAmountExhausted`, `DailyLimitExceeded`, `PerTxLimitExceeded`, `HourlyCapExceeded`, `NonceMismatch`, `Unauthorized`, `InvalidAmount`, `InvalidTtl`. `InvalidAddress` SHALL NOT be defined — `Pubkey` type enforcement makes it unreachable on-chain. `WhitelistAmountExhausted` is retained as a tombstone variant to preserve error code stability but is never emitted.

#### Scenario: All required error variants exist
- **WHEN** test enumerates `EnclzError` variants
- **THEN** every name in the spec error taxonomy is present and `InvalidAddress` is absent

#### Scenario: InvalidTtl is distinct from InvalidAmount
- **WHEN** `add_to_whitelist` is called with a past `ttl_expires_at`
- **THEN** the call fails with `InvalidTtl`, not `InvalidAmount`

#### Scenario: Anchor error code is stable
- **WHEN** test triggers any variant via `err!(EnclzError::DailyLimitExceeded)`
- **THEN** the resulting error carries a deterministic `error_code_number` (used by backend pass-through)

### Requirement: Constants module

The program SHALL expose seed-prefix constants (`GROUP_SEED`, `WALLET_SEED`, `WHITELIST_SEED`), default-limit constants (`DEFAULT_DAILY_LIMIT`, `DEFAULT_PER_TX_LIMIT`, `DEFAULT_HOURLY_CAP`), and `PROTOCOL_FEE_BPS = 10`.

#### Scenario: Constants match spec values
- **WHEN** test reads the constant values
- **THEN** seeds equal `b"group"` / `b"wallet"` / `b"whitelist"` and limits equal documented defaults
