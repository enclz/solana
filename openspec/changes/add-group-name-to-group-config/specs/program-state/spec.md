## MODIFIED Requirements

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
