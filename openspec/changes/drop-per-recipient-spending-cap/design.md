## Context

`WhitelistEntry` currently carries `approved_amount: u64` and `amount_used: u64` — a per-recipient spending cap incremented on every `execute_transfer` to an EXTERNAL recipient. This cap is enforced in addition to agent-level limits (`daily_limit`, `per_tx_limit`, `hourly_tx_cap`). It creates two problems:

1. **Cross-mint incoherence.** `WhitelistEntry` has no `mint` field, yet different agents in the same group can be bound to different mints (USDC 6-decimals, wSOL 9-decimals, etc.). Transfers to the same target from different agents accumulate `amount_used` in incommensurable units.
2. **Unnecessary complexity.** The per-recipient cap duplicates agent-level limits that already bound every transfer.

Removing the per-recipient cap simplifies the program, eliminates the cross-mint problem, and matches how most allowance systems work (cap the spender, not the recipient).

## Goals / Non-Goals

**Goals:**
- Remove `approved_amount` and `amount_used` from `WhitelistEntry` account layout
- Remove `approved_amount` parameter from `add_to_whitelist` and `renew_whitelist_entry`
- Remove per-recipient cap enforcement and auto-void from `execute_transfer`
- Preserve TTL enforcement for EXTERNAL entries
- Preserve error code stability (keep `WhitelistAmountExhausted` as tombstone)
- Keep PDA seeds unchanged — the whitelist is still keyed on `(group, target_address)`

**Non-Goals:**
- Adding a `mint` field to `WhitelistEntry` — no longer needed since there's no amount to track
- Changing PDA derivation
- Changing agent-level limit semantics
- Removing `entry_type` or `ttl_expires_at`

## Decisions

### Decision 1: Remove both fields, not just stop enforcing them

Removing `approved_amount` and `amount_used` from the struct reduces account rent (16 bytes saved) and eliminates dead data. The alternative — keeping the fields but ignoring them — wastes rent and leaves confusing state in the IDL.

### Decision 2: Keep `renew_whitelist_entry`, narrow it to TTL-only

`renew_whitelist_entry` currently updates both TTL and `approved_amount`. With the cap gone, renew still has one job: extend the expiration on an EXTERNAL entry. Removing the instruction entirely would force the owner to remove+re-add an entry just to extend its TTL, which is worse UX.

### Decision 3: Keep `WhitelistAmountExhausted` as a tombstone error variant

Removing the variant would shift all subsequent error discriminants (6002 → DailyLimitExceeded becomes 6002 instead of 6003, etc.), breaking the backend's error code mapping. The variant stays in the enum but is never emitted. A comment marks it as retired.

### Decision 4: Keep `entry_type` unchanged

Even without per-recipient caps, the three entry types still serve distinct purposes:
- `INTRA_GROUP` (0): automatically added by `add_agent`, non-removable
- `EXTERNAL` (1): owner-managed, TTL-enforced, removable
- `PROTOCOL` (2): owner-managed, permanent (no TTL), removable

### Decision 5: Keep TTL enforcement on EXTERNAL entries

A pure allowlist can still benefit from time-bounded access. An external merchant whitelisted for 24h should expire automatically without requiring owner action to remove. TTL provides defense-in-depth even without a spending cap.

## Risks / Trade-offs

- **Risk:** Without per-recipient caps, a compromised agent could drain its full daily limit to a single external address. **Mitigation:** The agent's `daily_limit` is the hard ceiling — the owner sets it per-agent, and it already bounds maximum exposure. Adding a second cap on the recipient side was redundant.
- **Risk:** Backend code that reads `approved_amount` / `amount_used` from the IDL will see those fields disappear. **Mitigation:** The backend repo is separate and can be updated independently. The on-chain IDL is the source of truth.
