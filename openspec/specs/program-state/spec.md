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

The program SHALL define a `WhitelistEntry` account derived from seeds `["whitelist", group_pubkey, target_address]` containing `target: Pubkey` (the target address, stored redundantly for read-side convenience), `label: [u8; 32]`, `added_by: Pubkey`, `entry_type: u8` (0=intra, 1=external, 2=protocol), `ttl_expires_at: i64`, `approved_amount: u64`, `amount_used: u64`, and a `bump: u8` storing the canonical PDA bump.

Account size: `8 + INIT_SPACE = 8 + 32 (target) + 32 (label) + 32 (added_by) + 1 (entry_type) + 8 (ttl_expires_at) + 8 (approved_amount) + 8 (amount_used) + 1 (bump) = 130 bytes`.

#### Scenario: PDA derivation matches spec
- **WHEN** test derives `[b"whitelist", group.as_ref(), target.as_ref()]`
- **THEN** result matches program-side PDA

#### Scenario: Entry type values match documented enum
- **WHEN** test reads the `entry_type` constants
- **THEN** `INTRA_GROUP == 0`, `EXTERNAL == 1`, `PROTOCOL == 2`

#### Scenario: target field round-trips unchanged
- **WHEN** test creates a `WhitelistEntry` with `target = arbitrary_pubkey`, serializes and deserializes
- **THEN** the decoded `target` equals the original pubkey

#### Scenario: Account size accommodates all fields including target
- **WHEN** test reads `WhitelistEntry::INIT_SPACE`
- **THEN** the value equals `32 + 32 + 32 + 1 + 8 + 8 + 8 + 1 = 122` and the total account allocation is `8 + 122 = 130`

### Requirement: Error enum mirrors backend codes

The program SHALL define error variants whose names map 1:1 to backend REST error codes: `WhitelistViolation`, `WhitelistExpired`, `WhitelistAmountExhausted`, `DailyLimitExceeded`, `PerTxLimitExceeded`, `HourlyCapExceeded`, `NonceMismatch`, `Unauthorized`, `InvalidAmount`, `InvalidTtl`, `TooManyAgents`, `InvalidMint`, `InvalidFeeAccount`, `InvalidTokenAccount`, `RecipientInvalid`, `InvalidEntryType`. `InvalidAddress` SHALL NOT be defined — `Pubkey` type enforcement makes it unreachable on-chain.

#### Scenario: All required error variants exist
- **WHEN** test enumerates `EnclzError` variants
- **THEN** every name in the spec error taxonomy is present and `InvalidAddress` is absent

#### Scenario: InvalidEntryType is distinct from Unauthorized
- **WHEN** `add_to_whitelist` is called with `entry_type == 0` or an unrecognized entry type
- **THEN** the call fails with `InvalidEntryType`, not `Unauthorized`

#### Scenario: RecipientInvalid error code is 6014
- **WHEN** test reads `EnclzError::RecipientInvalid as u32`
- **THEN** the value equals 14 (Anchor maps to code 6014)

#### Scenario: InvalidEntryType error code is 6015
- **WHEN** test reads `EnclzError::InvalidEntryType as u32`
- **THEN** the value equals 15 (Anchor maps to code 6015)

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
