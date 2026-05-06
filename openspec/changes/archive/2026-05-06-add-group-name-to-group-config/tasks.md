## 1. Program changes

- [x] 1.1 Append `pub group_name: [u8; 32],` after `agent_count` in `programs/enclz/src/state/group_config.rs`
- [x] 1.2 In `programs/enclz/src/instructions/initialize_group.rs`, update the `#[instruction(...)]` attribute (line 8) and the `handle_initialize_group` signature (lines 34–39) so `group_name: [u8; 32]` is the **first** argument, before `backend_operator`
- [x] 1.3 In the same handler, write `group_config.group_name = group_name;` after the existing field assignments (around line 44). Do not validate the bytes
- [x] 1.4 Confirm that `space = GroupConfig::DISCRIMINATOR.len() + GroupConfig::INIT_SPACE` at `initialize_group.rs:16` is unchanged (Anchor auto-derives the new size)
- [x] 1.5 Update the `#[program]` wrapper `initialize_group` in `programs/enclz/src/lib.rs:32–44` to thread `group_name` to the handler (not in original task list — discovered during compile)

## 2. Rust unit + litesvm tests

- [x] 2.1 Update `init_space_group_config_matches_field_layout` in `programs/enclz/src/lib.rs` so the assertion reads `32 + 32 + 32 + 1 + 32` and the comment lists `group_name`
- [x] 2.2 Update `group_config_round_trip_through_init_space_buffer` to set `group_name` to a non-zero pattern (`[42u8; 32]`) and assert byte-equality after decode
- [x] 2.3 Update `programs/enclz/tests/common/mod.rs` `initialize_group_instruction(...)` builder + `provision_group_with_router(...)` helper + the direct call in `programs/enclz/tests/owner_instructions.rs:60` (not in original task list — discovered during compile)
- [x] 2.4 Add Rust litesvm test `initialize_group_stores_group_name_verbatim_including_non_utf8_bytes` covering the spec scenario "Non-UTF-8 name accepted"
- [x] 2.5 Run `cargo test --package enclz` — 24 passed, 0 failed

## 3. TypeScript integration tests

- [x] 3.1 Reused existing per-file `padDisplayName(text)` helper (already 32-byte buffer producer) — no new shared helper needed
- [x] 3.2 `tests/owner_instructions.spec.ts` — `provisionGroup` now takes a `groupName` param defaulting to `padDisplayName("acme-trading-desk")`
- [x] 3.3 `tests/smoke.ts` — passes `padDisplayName("smoke-test-group")`
- [x] 3.4 `tests/execute_swap.spec.ts` — passes `padDisplayName("swap-test")`
- [x] 3.5 `tests/execute_lending_op.spec.ts` — both call sites pass `padDisplayName("lending-test")`
- [x] 3.6 `tests/execute_transfer.spec.ts` — `provisionFleet` passes `padDisplayName("transfer-test")`
- [x] 3.7 `tests/owner_instructions.spec.ts` asserts `Array.from(groupConfig.groupName)` deep-equals the input
- [x] 3.8 `npm run test:e2e` — 7 passing

## 4. Version bump and security_txt

- [x] 4.1 `programs/enclz/Cargo.toml` `version` 0.1.2 → 0.2.0
- [x] 4.2 `programs/enclz/src/lib.rs` `source_release` "v0.1.2" → "v0.2.0"
- [x] 4.3 `anchor build` regenerated `target/idl/enclz.json` with `metadata.version: "0.2.0"`, `initialize_group` first arg `group_name: [u8; 32]`, `GroupConfig` field listed
- [x] 4.4 `node scripts/check-idl-coverage.mjs` — 11 handlers / 11 instructions, in sync
- [x] 4.5 `node scripts/build-sdk.mjs` — `sdk/package.json` bumped to 0.2.0, `sdk/dist/` rebuilt

## 5. Documentation

- [x] 5.1 `docs/SPECIFICATION.md` `GroupConfig` field listing now includes `group_name: [u8; 32]`; `initialize_group` Args block now lists `group_name` first
- [x] 5.2 No hardcoded `97`/`129`/`INIT_SPACE` size found in `docs/SPECIFICATION.md` — nothing to update
- [x] 5.3 Docs change committed on submodule branch `feat/add-group-name-to-group-config`; remote push deferred to a follow-up

## 6. Devnet redeploy

- [x] 6.1 PATH exported for anchor + sbf tools
- [x] 6.2 `npm run deploy:devnet` — needed `solana program extend 4096` first (binary grew 472 bytes); deploy then upgraded program to 0.2.0 and refreshed on-chain IDL metadata. Followed up with `npm run idl:upgrade:devnet` for belt-and-suspenders. Smoke test (`npm run smoke:devnet`, public devnet RPC) passed end-to-end.
- [x] 6.3 Devnet round-trip verified: initialised group with `group_name = "acme-trading-desk"`, `program.account.groupConfig.fetch(...)` returned the same 32 bytes; UTF-8 trim decoded back to the input string. SDK published as `@enclz/sdk@0.2.0`.

## 7. Archive

- [x] 7.1 Archived via `openspec archive add-group-name-to-group-config` — moved to `openspec/changes/archive/2026-05-06-add-group-name-to-group-config/`; spec deltas applied to `openspec/specs/program-state/` and `openspec/specs/group-provisioning/`
