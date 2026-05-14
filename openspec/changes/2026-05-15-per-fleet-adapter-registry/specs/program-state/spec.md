## ADDED Requirements

### Requirement: GroupAdapterRegistry PDA

The program SHALL define a `GroupAdapterRegistry` account derived from seeds `["adapter_registry", group_config_pda]` containing `group_config: Pubkey` (back-reference to the owning `GroupConfig`), `bump: u8`, and `entries: Vec<AdapterEntry>` with a hard cap of 32 entries.

Each `AdapterEntry` contains `program_id: Pubkey`, `label: [u8; 32]` (UTF-8, fixed-width, written verbatim with no on-chain encoding validation), `status: u8` (0 = Active, 1 = Paused), `constraints: Vec<u8>` (opaque to the core program; passed verbatim to the adapter on every `execute_via_adapter` call), and `added_at: i64` (unix seconds, as recorded by the runtime clock at insertion time).

The registry is created lazily — `add_adapter` uses `init_if_needed` so the first call materializes the PDA and pays rent; subsequent calls write into the existing account. The registry is not closed when emptied — `remove_adapter` leaves an empty `entries: Vec<>` and the account remains rent-paid.

#### Scenario: PDA derivation matches spec
- **WHEN** test derives `[b"adapter_registry", group_config_pda.as_ref()]`
- **THEN** result matches program-side PDA

#### Scenario: AdapterStatus values match documented enum
- **WHEN** test reads the `AdapterStatus` constants
- **THEN** `ACTIVE == 0` and `PAUSED == 1`

#### Scenario: INIT_SPACE accommodates header + zero entries
- **WHEN** test reads `GroupAdapterRegistry::INIT_SPACE`
- **THEN** the value equals `32 (group_config) + 1 (bump) + 4 (Vec length prefix) = 37` for the empty case

#### Scenario: Max entries enforced
- **WHEN** an `add_adapter` call would push `entries.len()` past 32
- **THEN** the instruction fails with `AdapterRegistryFull`

#### Scenario: Back-reference matches owning group
- **WHEN** test reads `GroupAdapterRegistry.group_config`
- **THEN** the value equals the `group_config_pda` from the seeds
