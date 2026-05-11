## Why

The `WhitelistEntry` tracks `approved_amount` and `amount_used` — a per-recipient spending cap. This duplicates the agent-level limits (`daily_limit`, `per_tx_limit`, `hourly_tx_cap`) already enforced on every transfer, and creates cross-mint incoherence because the entry has no `mint` field: different agents sending different tokens to the same recipient accumulate `amount_used` in incommensurable units. The simplest correct model is a pure allowlist — recipient is either approved or not — with spending bounded solely by the agent's own limits.

## What Changes

- **BREAKING:** Remove `approved_amount` and `amount_used` fields from `WhitelistEntry` account struct
- **BREAKING:** Remove `approved_amount` parameter from `add_to_whitelist` instruction; `ttl_expires_at` remains for EXTERNAL entries
- **BREAKING:** Remove `approved_amount` parameter from `renew_whitelist_entry` instruction; only TTL is renewed
- Remove per-recipient cap check and `amount_used` increment from `execute_transfer`; keep TTL expiry check for EXTERNAL entries
- Remove auto-void (PDA close) on cap exhaustion — no cap to exhaust
- Keep `WhitelistAmountExhausted` error variant as a tombstone to preserve error code stability
- Agent-level limits (`daily_limit`, `per_tx_limit`, `hourly_tx_cap`) become the sole spending constraint

## Capabilities

### New Capabilities

None — this change removes constraints, not adds them.

### Modified Capabilities

- `program-state`: `WhitelistEntry` account layout changes — two `u64` fields removed
- `whitelist-management`: `add_to_whitelist` and `renew_whitelist_entry` signatures change; `approved_amount` validation removed
- `transfer-execution`: per-recipient cap check and auto-void on exhaustion removed; TTL enforcement unchanged

## Impact

- Program: `programs/enclz/src/state/whitelist_entry.rs`, `instructions/add_to_whitelist.rs`, `instructions/renew_whitelist_entry.rs`, `instructions/execute_transfer.rs`, `instructions/add_agent.rs`, `instructions/initialize_group.rs`, `lib.rs`, `errors.rs`
- Tests: `tests/owner_instructions.spec.ts`, `tests/execute_transfer.spec.ts`, `tests/smoke.ts`, `tests/execute_swap.spec.ts`, `tests/execute_lending_op.spec.ts`
- Unit tests in `lib.rs`: `init_space_whitelist_entry_matches_field_layout`, `whitelist_entry_round_trip_through_init_space_buffer`
- Docs: `docs/SPECIFICATION.md`
- OpenSpec specs: `program-state/spec.md`, `whitelist-management/spec.md`, `transfer-execution/spec.md`
- SDK IDL regenerates via `anchor build`
- Backend: `WhitelistAmountExhausted` error will never be emitted; `approved_amount` / `amount_used` fields disappear from IDL
