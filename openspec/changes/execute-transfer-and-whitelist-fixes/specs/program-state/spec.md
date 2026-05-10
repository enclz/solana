# program-state Specification Delta

## MODIFIED Requirements

### Requirement: WhitelistEntry PDA

The program SHALL define a `WhitelistEntry` account derived from seeds `["whitelist", group_pubkey, target_address]` containing `target: Pubkey` (the target address, stored redundantly for read-side convenience), `label`, `added_by` pubkey, `entry_type: u8` (0=intra, 1=external, 2=protocol), `ttl_expires_at: i64`, `approved_amount: u64`, `amount_used: u64`, and a `bump: u8` storing the canonical PDA bump.

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

#### Scenario: Anchor error code is stable
- **WHEN** test triggers any variant via `err!(EnclzError::DailyLimitExceeded)`
- **THEN** the resulting error carries a deterministic `error_code_number` (used by backend pass-through)

### Requirement: Constants module

The program SHALL expose seed-prefix constants (`GROUP_SEED`, `WALLET_SEED`, `WHITELIST_SEED`), default-limit constants (`DEFAULT_DAILY_LIMIT`, `DEFAULT_PER_TX_LIMIT`, `DEFAULT_HOURLY_CAP`), and `PROTOCOL_FEE_BPS = 10`.

#### Scenario: Constants match spec values
- **WHEN** test reads the constant values
- **THEN** seeds equal `b"group"` / `b"wallet"` / `b"whitelist"` and limits equal documented defaults
