# whitelist-management Specification Delta

## MODIFIED Requirements

### Requirement: add_to_whitelist instruction

The program SHALL expose `add_to_whitelist(target_address: Pubkey, label: [u8;32], entry_type: u8, ttl_expires_at: i64, approved_amount: u64)` callable only by the group owner. It creates a `WhitelistEntry` PDA with `target` set to `target_address` and validates type-specific invariants.

#### Scenario: External entry with valid TTL and amount
- **WHEN** owner calls with `entry_type == 1`, `ttl_expires_at` in the future, `approved_amount > 0`
- **THEN** the PDA is created with `target == target_address`, the provided values, and `amount_used == 0`

#### Scenario: Intra-group entry type rejected
- **WHEN** owner calls `add_to_whitelist` with `entry_type == 0`
- **THEN** the call fails with `InvalidEntryType` — intra-group entries are created exclusively by `add_agent`

#### Scenario: External entry rejects past TTL
- **WHEN** owner calls with `entry_type == 1` and `ttl_expires_at <= now`
- **THEN** the call fails with `InvalidTtl`

#### Scenario: External entry rejects zero approved amount
- **WHEN** owner calls with `entry_type == 1` and `approved_amount == 0`
- **THEN** the call fails

#### Scenario: Unknown entry type rejected
- **WHEN** owner calls with an `entry_type` value other than 0, 1, or 2
- **THEN** the call fails with `InvalidEntryType`

#### Scenario: Protocol entry forces zero TTL/amount
- **WHEN** owner calls with `entry_type == 2`
- **THEN** the stored entry has `target == target_address`, `ttl_expires_at == 0`, and `approved_amount == 0` regardless of input

#### Scenario: Non-owner rejected
- **WHEN** any non-owner signer calls `add_to_whitelist`
- **THEN** the call fails with `Unauthorized`

### Requirement: remove_from_whitelist instruction

The program SHALL expose `remove_from_whitelist()` callable only by the group owner. It closes the `WhitelistEntry` PDA and returns rent to the owner. Intra-group entries (`entry_type == 0`) cannot be removed.

#### Scenario: Successful removal of external entry
- **WHEN** owner calls `remove_from_whitelist` against an `entry_type == 1` PDA
- **THEN** the PDA is closed, lamports return to the owner, and subsequent transfers to that address fail with `AccountNotInitialized` (3012; translated to `whitelist_violation` by the backend)

#### Scenario: Successful removal of protocol entry
- **WHEN** owner calls `remove_from_whitelist` against an `entry_type == 2` PDA
- **THEN** the PDA is closed

#### Scenario: Reject intra-group removal
- **WHEN** owner calls `remove_from_whitelist` against an `entry_type == 0` PDA
- **THEN** the call fails

## ADDED Requirements

### Requirement: target field stored on WhitelistEntry

The program SHALL store the `target_address` argument of `add_to_whitelist` as the `target: Pubkey` field on the `WhitelistEntry` account. For intra-group entries created by `add_agent`, the `target` SHALL be set to `agent_wallet.key()`.

#### Scenario: add_to_whitelist stores target
- **WHEN** owner calls `add_to_whitelist` with `target_address = X`
- **THEN** the created `WhitelistEntry` has `target == X`

#### Scenario: add_agent stores agent PDA as target
- **WHEN** owner calls `add_agent` and the intra-group entry is auto-created
- **THEN** the intra-group `WhitelistEntry` has `target == agent_wallet.key()`
