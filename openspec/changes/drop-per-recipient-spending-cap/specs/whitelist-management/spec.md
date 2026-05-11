## MODIFIED Requirements

### Requirement: add_to_whitelist instruction

The program SHALL expose `add_to_whitelist(target_address: Pubkey, label: [u8;32], entry_type: u8, ttl_expires_at: i64)` callable only by the group owner. It creates a `WhitelistEntry` PDA and validates type-specific invariants.

#### Scenario: External entry with valid TTL
- **WHEN** owner calls with `entry_type == 1` and `ttl_expires_at` in the future
- **THEN** the PDA is created with the provided `label`, `target_address`, `entry_type`, and `ttl_expires_at`

#### Scenario: External entry rejects past TTL
- **WHEN** owner calls with `entry_type == 1` and `ttl_expires_at <= now`
- **THEN** the call fails with `InvalidTtl`

#### Scenario: Intra-group entry type rejected
- **WHEN** owner calls `add_to_whitelist` with `entry_type == 0`
- **THEN** the call fails with `InvalidEntryType` — intra-group entries are created exclusively by `add_agent`

#### Scenario: Permanent entry forces zero TTL
- **WHEN** owner calls with `entry_type == 2`
- **THEN** the stored entry has `ttl_expires_at == 0` regardless of input

#### Scenario: Non-owner rejected
- **WHEN** any non-owner signer calls `add_to_whitelist`
- **THEN** the call fails with `Unauthorized`

### Requirement: renew_whitelist_entry instruction

The program SHALL expose `renew_whitelist_entry(target_address: Pubkey, ttl_expires_at: i64)` for `entry_type == 1` PDAs only, callable only by the group owner. It mutates the existing PDA's `ttl_expires_at` in place.

#### Scenario: Successful renewal
- **WHEN** owner calls with `ttl_expires_at > now`
- **THEN** the PDA's `ttl_expires_at` is updated; PDA address unchanged

#### Scenario: Reject past TTL
- **WHEN** owner calls with `ttl_expires_at <= now`
- **THEN** the call fails with `InvalidTtl`

#### Scenario: Reject on intra-group entry
- **WHEN** owner calls `renew_whitelist_entry` against an `entry_type == 0` PDA
- **THEN** the call fails with `Unauthorized`

#### Scenario: Reject on protocol entry
- **WHEN** owner calls `renew_whitelist_entry` against an `entry_type == 2` PDA
- **THEN** the call fails with `Unauthorized`

## REMOVED Requirements

### Requirement: renew_whitelist_entry approved_amount guard
**Reason**: `approved_amount` and `amount_used` fields are removed. The per-recipient spending cap no longer exists.
**Migration**: Renewal now only updates `ttl_expires_at`. No replacement validation needed.
