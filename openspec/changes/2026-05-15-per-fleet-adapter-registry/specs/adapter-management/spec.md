## ADDED Requirements

### Requirement: add_adapter instruction signature and account constraints

The program SHALL expose `add_adapter(program_id: Pubkey, label: [u8; 32], constraints: Vec<u8>)` callable only by the `owner` recorded on the agent's `GroupConfig`. The instruction appends an `AdapterEntry` to the `GroupAdapterRegistry` for the calling group.

Required accounts: `owner_signer` (signer, address-bound to `group_config.owner`), `group_config`, `adapter_registry` (writable, `init_if_needed` from seeds `["adapter_registry", group_config.key()]`), `system_program`.

The instruction SHALL:
- Reject if `entries.len() >= 32` with `AdapterRegistryFull`
- Reject if an entry with the same `program_id` already exists with `Active` status, with `AdapterAlreadyRegistered`
- Allow re-adding a `Paused` entry — promote it to `Active` and overwrite `label`, `constraints`, `added_at`
- Set `status = Active` on insertion
- Set `added_at` to the runtime clock unix timestamp at insertion time

#### Scenario: Non-owner signer rejected
- **WHEN** any signer other than `GroupConfig.owner` invokes `add_adapter`
- **THEN** the call fails with `Unauthorized`

#### Scenario: First call materializes the registry PDA
- **WHEN** `add_adapter` is called against a group that has never had an adapter
- **THEN** the `GroupAdapterRegistry` PDA is created, owner pays rent, and `entries` contains the one new entry

#### Scenario: Duplicate Active program_id rejected
- **WHEN** an active entry with the same `program_id` is already in the registry
- **THEN** the call fails with `AdapterAlreadyRegistered`

#### Scenario: Re-adding a Paused entry promotes it back to Active
- **WHEN** an entry with the given `program_id` exists with `status == Paused` and `add_adapter` is called for the same `program_id`
- **THEN** the existing entry is updated in place — `status` becomes `Active`, `label` and `constraints` are overwritten, `added_at` is refreshed

#### Scenario: Over-cap insertion rejected
- **WHEN** the registry already holds 32 entries (all Active or mixed) and `add_adapter` is called
- **THEN** the call fails with `AdapterRegistryFull`

#### Scenario: Constraints are stored verbatim
- **WHEN** `add_adapter` is called with any well-formed `Vec<u8>` for `constraints`
- **THEN** the registry entry stores those exact bytes for retrieval on subsequent `execute_via_adapter` calls

### Requirement: remove_adapter instruction signature and account constraints

The program SHALL expose `remove_adapter(program_id: Pubkey)` callable only by the `owner` recorded on the agent's `GroupConfig`. The instruction removes the matching `AdapterEntry` from the `GroupAdapterRegistry`.

Required accounts: `owner_signer` (signer, address-bound to `group_config.owner`), `group_config`, `adapter_registry` (writable), `system_program`.

The instruction SHALL:
- Locate the entry by exact `program_id` match
- Delete the entry (compact `entries` in place) if found
- Succeed silently if no matching entry is present (idempotent removal)

#### Scenario: Non-owner signer rejected
- **WHEN** any signer other than `GroupConfig.owner` invokes `remove_adapter`
- **THEN** the call fails with `Unauthorized`

#### Scenario: Existing entry removed
- **WHEN** `remove_adapter` is called against a `program_id` present in the registry
- **THEN** the entry is removed; `entries.len()` decreases by 1

#### Scenario: Missing entry is a no-op
- **WHEN** `remove_adapter` is called against a `program_id` not in the registry
- **THEN** the instruction succeeds with no state change (idempotent)

#### Scenario: Registry PDA persists after last entry removed
- **WHEN** the last entry is removed via `remove_adapter`
- **THEN** the `GroupAdapterRegistry` PDA remains rent-paid with `entries.len() == 0`; the owner can `add_adapter` again without re-initializing
