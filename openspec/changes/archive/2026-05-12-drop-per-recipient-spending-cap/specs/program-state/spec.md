## MODIFIED Requirements

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
