## Why

Backend tooling and the upcoming webapp need a stable, human-readable label per orchestrator group so dashboards, onboarding, and audit trails can show "Acme Trading Desk" instead of a base58 PDA. The label has to live onchain (single source of truth, survives backend restarts) and round-trip through the IDL so the SDK surfaces it without bespoke decoders.

## What Changes

- **BREAKING** `GroupConfig` gains a `group_name: [u8; 32]` field appended after `agent_count`. `INIT_SPACE` rises from 97 → 129 bytes; previously-deployed `GroupConfig` accounts on devnet cannot be decoded by the new program (acceptable — devnet is greenfield, no production tenants).
- **BREAKING** `initialize_group` signature gains `group_name: [u8; 32]` as the **first** argument: `initialize_group(group_name, backend_operator, protocol_fee_wallet, dex_router)`. Bytes are written verbatim — no on-chain UTF-8 validation; backend owns padding and input hygiene.
- Program version bumps `0.1.2 → 0.2.0`; `security_txt!` `source_release` bumps in lockstep. SDK `0.1.x → 0.2.0` follows automatically through the existing `scripts/build-sdk.mjs` pipeline.
- No new instructions (no `update_group_name`), no new errors, no realloc helper.

## Capabilities

### New Capabilities
<!-- None — this change extends existing capabilities. -->

### Modified Capabilities
- `program-state`: `GroupConfig PDA` requirement extended to include the `group_name: [u8; 32]` field and the new INIT_SPACE size.
- `group-provisioning`: `initialize_group instruction` requirement updated so the signature takes `group_name` as the first argument and writes it to the new `GroupConfig` field.

## Impact

- **Onchain account layout**: `GroupConfig` size changes; any tooling that hand-computes the size (none in this repo — Anchor `InitSpace` is the only path) would break. Existing devnet groups must be re-initialized after redeploy.
- **Program ABI**: `initialize_group` discriminator is unchanged but its argument vector grows by 32 bytes at the front. SDK consumers must pass the new arg.
- **SDK**: `@enclz/sdk` cuts `0.2.0`. Anchor IDL → TypeScript surfaces `groupConfig.groupName` as `number[]` (length 32) automatically; consumers may want a `decodeFixed32` helper, but that lives in the webapp repo, not here.
- **Tests**: 5 TS integration spec call sites for `.initializeGroup(...)` need the new first argument; Rust unit tests pinning `INIT_SPACE` and the round-trip serializer need the new field.
- **Docs**: `docs/SPECIFICATION.md` GroupConfig field listing.
- **Backend**: separate repo — must adopt the new arg before calling `initialize_group` against devnet `0.2.0`.
