## ADDED Requirements

### Requirement: add_to_whitelist instruction

The program SHALL expose `add_to_whitelist(target_address: Pubkey, label: [u8;32], entry_type: u8, ttl_expires_at: i64, approved_amount: u64)` callable only by the group owner. It creates a `WhitelistEntry` PDA and validates type-specific invariants.

#### Scenario: External entry with valid TTL and amount
- **WHEN** owner calls with `entry_type == 1`, `ttl_expires_at` in the future, `approved_amount > 0`
- **THEN** the PDA is created with the provided values and `amount_used == 0`

#### Scenario: Intra-group entry type rejected
- **WHEN** owner calls `add_to_whitelist` with `entry_type == 0`
- **THEN** the call fails with `Unauthorized` — intra-group entries are created exclusively by `add_agent`

#### Scenario: External entry rejects past TTL
- **WHEN** owner calls with `entry_type == 1` and `ttl_expires_at <= now`
- **THEN** the call fails with `InvalidTtl`

#### Scenario: External entry rejects zero approved amount
- **WHEN** owner calls with `entry_type == 1` and `approved_amount == 0`
- **THEN** the call fails

#### Scenario: Permanent entry forces zero TTL/amount
- **WHEN** owner calls with `entry_type == 0` or `entry_type == 2`
- **THEN** the stored entry has `ttl_expires_at == 0` and `approved_amount == 0` regardless of input

#### Scenario: Non-owner rejected
- **WHEN** any non-owner signer calls `add_to_whitelist`
- **THEN** the call fails with `Unauthorized`

### Requirement: renew_whitelist_entry instruction

The program SHALL expose `renew_whitelist_entry(ttl_expires_at: i64, approved_amount: u64)` for `entry_type == 1` PDAs only, callable only by the group owner. It mutates the existing PDA in place.

#### Scenario: Successful renewal
- **WHEN** owner calls with `ttl_expires_at > now` and `approved_amount >= current amount_used`
- **THEN** the PDA's `ttl_expires_at` and `approved_amount` are updated; PDA address unchanged

#### Scenario: Reject past TTL
- **WHEN** owner calls with `ttl_expires_at <= now`
- **THEN** the call fails with `InvalidTtl`

#### Scenario: Reject lowering cap below consumed
- **WHEN** owner calls with `approved_amount < amount_used`
- **THEN** the call fails

#### Scenario: Reject on intra-group entry
- **WHEN** owner calls `renew_whitelist_entry` against an `entry_type == 0` PDA
- **THEN** the call fails

#### Scenario: Reject on protocol entry
- **WHEN** owner calls `renew_whitelist_entry` against an `entry_type == 2` PDA
- **THEN** the call fails

### Requirement: remove_from_whitelist instruction

The program SHALL expose `remove_from_whitelist()` callable only by the group owner. It closes the `WhitelistEntry` PDA and returns rent to the owner. Intra-group entries (`entry_type == 0`) cannot be removed.

#### Scenario: Successful removal of external entry
- **WHEN** owner calls `remove_from_whitelist` against an `entry_type == 1` PDA
- **THEN** the PDA is closed, lamports return to the owner, and subsequent transfers to that address fail with `WhitelistViolation`

#### Scenario: Successful removal of protocol entry
- **WHEN** owner calls `remove_from_whitelist` against an `entry_type == 2` PDA
- **THEN** the PDA is closed

#### Scenario: Reject intra-group removal
- **WHEN** owner calls `remove_from_whitelist` against an `entry_type == 0` PDA
- **THEN** the call fails
