## 1. Program changes

- [ ] 1.1 Append `pub group_name: [u8; 32],` after `agent_count` in `programs/enclz/src/state/group_config.rs`
- [ ] 1.2 In `programs/enclz/src/instructions/initialize_group.rs`, update the `#[instruction(...)]` attribute (line 8) and the `handle_initialize_group` signature (lines 34–39) so `group_name: [u8; 32]` is the **first** argument, before `backend_operator`
- [ ] 1.3 In the same handler, write `group_config.group_name = group_name;` after the existing field assignments (around line 44). Do not validate the bytes
- [ ] 1.4 Confirm that `space = GroupConfig::DISCRIMINATOR.len() + GroupConfig::INIT_SPACE` at `initialize_group.rs:16` is unchanged (Anchor auto-derives the new size)

## 2. Rust unit tests

- [ ] 2.1 Update `init_space_group_config_matches_field_layout` in `programs/enclz/src/lib.rs:230–233` so the assertion reads `32 + 32 + 32 + 1 + 32` and the comment lists `group_name`
- [ ] 2.2 Update `group_config_round_trip_through_init_space_buffer` in `programs/enclz/src/lib.rs:249–267` to set `group_name` to a non-zero pattern (e.g. `[42u8; 32]`) on the input value and assert the decoded `group_name` is byte-equal
- [ ] 2.3 Run `cargo test --package enclz` and confirm all tests pass

## 3. TypeScript integration tests

- [ ] 3.1 Add a small helper that produces a 32-byte name (`Buffer.alloc(32).copy(Buffer.from(name, 'utf8'))` returned as `number[]`). Place it where the test files can share it (or inline per spec — match existing style)
- [ ] 3.2 Update the `.initializeGroup(...)` call in `tests/owner_instructions.spec.ts:108` to pass the name first
- [ ] 3.3 Update the `.initializeGroup(...)` call in `tests/smoke.ts:215`
- [ ] 3.4 Update the `.initializeGroup(...)` call in `tests/execute_swap.spec.ts:108`
- [ ] 3.5 Update the `.initializeGroup(...)` calls in `tests/execute_lending_op.spec.ts:113` and `:235`
- [ ] 3.6 Update the `.initializeGroup(...)` call in `tests/execute_transfer.spec.ts:114`
- [ ] 3.7 In `tests/owner_instructions.spec.ts`, add a post-init assertion that `groupConfig.groupName` (decoded as `number[]`) byte-equals the input
- [ ] 3.8 Run `npm run test:e2e` and confirm all specs pass

## 4. Version bump and security_txt

- [ ] 4.1 Bump `programs/enclz/Cargo.toml` `version` from `0.1.2` to `0.2.0`
- [ ] 4.2 Bump `programs/enclz/src/lib.rs:24` `source_release` from `"v0.1.2"` to `"v0.2.0"`
- [ ] 4.3 Run `anchor build` and confirm `target/idl/enclz.json` regenerates with `metadata.version == "0.2.0"`, `instructions[].name == "initialize_group"` lists `group_name` as the first arg of type `{ "array": ["u8", 32] }`, and `accounts[].name == "GroupConfig"` includes `group_name`
- [ ] 4.4 Run `node scripts/check-idl-coverage.mjs` (the same gate CI runs after `anchor build`)
- [ ] 4.5 Run `node scripts/build-sdk.mjs` and confirm `sdk/package.json` `version` updates to `"0.2.0"`

## 5. Documentation

- [ ] 5.1 Update `docs/SPECIFICATION.md` GroupConfig field listing (lines ~64–71) to include `group_name: [u8; 32]` after `agent_count`
- [ ] 5.2 If `docs/SPECIFICATION.md` records the account size anywhere downstream, update `97 → 129` bytes
- [ ] 5.3 Follow the docs-submodule push protocol from `CLAUDE.md` (SSH remote, no `--remote --merge` while a feature branch is checked out in the submodule) when committing the docs change

## 6. Devnet redeploy

- [ ] 6.1 Export PATH for anchor + sbf tools: `export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"`
- [ ] 6.2 Run `npm run deploy:devnet`
- [ ] 6.3 Manually re-init at least one test group via the SDK with a non-empty name; fetch the account and confirm `groupName` decodes to the bytes that were sent

## 7. Archive

- [ ] 7.1 Once devnet smoke passes, archive the change directory under `openspec/changes/archive/` per repo convention (the `openspec-archive-change` skill or manual move)
