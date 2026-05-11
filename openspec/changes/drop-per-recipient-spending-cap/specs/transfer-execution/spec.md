## MODIFIED Requirements

### Requirement: Whitelist enforcement for EXTERNAL recipients

For `entry_type == 1` (EXTERNAL), the program SHALL enforce TTL expiry: the current clock timestamp MUST be less than or equal to `whitelist_entry.ttl_expires_at`. No per-recipient spending cap is enforced — the agent's own limits (`daily_limit`, `per_tx_limit`, `hourly_tx_cap`) are the sole spending constraints.

#### Scenario: Transfer succeeds within TTL
- **WHEN** backend operator calls `execute_transfer` to an EXTERNAL recipient with a whitelist entry where `ttl_expires_at >= now`
- **THEN** the transfer succeeds (subject to agent-level limits)

#### Scenario: Transfer fails when TTL expired
- **WHEN** backend operator calls `execute_transfer` to an EXTERNAL recipient with a whitelist entry where `ttl_expires_at < now`
- **THEN** the call fails with `WhitelistExpired`

#### Scenario: Transfer fails when whitelist entry missing
- **WHEN** backend operator calls `execute_transfer` to a recipient address with no `WhitelistEntry` PDA
- **THEN** Anchor account resolution fails (PDA not found), surfacing as `WhitelistViolation`

#### Scenario: Transfer to EXTERNAL recipient succeeds regardless of transfer count
- **WHEN** backend operator calls `execute_transfer` multiple times to the same EXTERNAL recipient, all within TTL
- **THEN** each transfer succeeds independently, bounded only by agent-level limits

## REMOVED Requirements

### Requirement: Whitelist amount cap enforcement for EXTERNAL recipients
**Reason**: `approved_amount` and `amount_used` fields are removed from `WhitelistEntry`. Per-recipient spending caps duplicate agent-level limits and are meaningless across different token mints without a `mint` field on the entry.
**Migration**: Agent-level limits (`daily_limit`, `per_tx_limit`) are the sole spending constraints. No replacement cap is needed.

### Requirement: Whitelist amount_used increment
**Reason**: `amount_used` field is removed. There is no per-recipient consumption to track.
**Migration**: No replacement — whitelist entries are not mutated by `execute_transfer`.

### Requirement: Auto-void on whitelist exhaustion
**Reason**: Without `approved_amount` and `amount_used`, there is no concept of exhaustion. The whitelist entry PDA is never closed automatically by `execute_transfer`.
**Migration**: To revoke a whitelist entry, the owner calls `remove_from_whitelist`.
