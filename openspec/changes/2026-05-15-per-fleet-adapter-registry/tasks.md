## 1. State

- [ ] 1.1 Add `programs/enclz/src/state/adapter_registry.rs` with `GroupAdapterRegistry` and `AdapterEntry` structs, `INIT_SPACE`/`MAX_ENTRIES` constants
- [ ] 1.2 Register the new module in `programs/enclz/src/state/mod.rs`
- [ ] 1.3 Add `init_space_adapter_registry_matches_field_layout` and `adapter_registry_round_trip_through_init_space_buffer` unit tests in `programs/enclz/src/lib.rs`

## 2. Errors

- [ ] 2.1 Add `AdapterNotApproved`, `AdapterAlreadyRegistered`, `AdapterRegistryFull` variants to `programs/enclz/src/errors.rs` using the next free 6000-band discriminants
- [ ] 2.2 Update `error_variants_have_stable_codes` unit test with new discriminants

## 3. add_adapter / remove_adapter instructions

- [ ] 3.1 Add `programs/enclz/src/instructions/add_adapter.rs` — `init_if_needed` for `GroupAdapterRegistry` PDA, owner-signed, append entry, reject duplicates + over-cap
- [ ] 3.2 Add `programs/enclz/src/instructions/remove_adapter.rs` — owner-signed, remove entry by `program_id`, idempotent on missing
- [ ] 3.3 Register both in `programs/enclz/src/instructions/mod.rs` and the `#[program]` block in `lib.rs`

## 4. execute_via_adapter instruction

- [ ] 4.1 Add `programs/enclz/src/instructions/execute_via_adapter.rs` — operator-signed, validates `adapter_id` is in `GroupAdapterRegistry.entries` with `status == Active`, applies amount cap + frequency policy checks identical to `execute_transfer`
- [ ] 4.2 Implement custody post-check: every writable token account in `remaining_accounts` that the CPI touches must be PDA-owned by the agent wallet
- [ ] 4.3 Forbid Token, ATA, and BPF loader program IDs as CPI targets (defense-in-depth alongside the registry pre-check)
- [ ] 4.4 Invoke target adapter via `invoke_signed` with the agent_wallet PDA seeds, passing `call_data` and `constraints` from the registry entry
- [ ] 4.5 Register in `programs/enclz/src/instructions/mod.rs` and the `#[program]` block

## 5. Removed instructions

- [ ] 5.1 Delete `programs/enclz/src/instructions/execute_swap.rs`
- [ ] 5.2 Delete `programs/enclz/src/instructions/execute_lending_op.rs`
- [ ] 5.3 Remove their registrations from `instructions/mod.rs` and the `#[program]` block in `lib.rs`
- [ ] 5.4 Delete corresponding Rust integration test files: `programs/enclz/tests/execute_swap.rs`, `programs/enclz/tests/execute_lending_op.rs`
- [ ] 5.5 Update `programs/enclz/tests/common/mod.rs` helpers — drop swap/lending fixtures

## 6. Integration tests

- [ ] 6.1 Add `tests/adapter_management.spec.ts` — happy paths + invalid signer + duplicate + over-cap + remove-then-readd + remove-while-not-present (idempotent)
- [ ] 6.2 Add `tests/execute_via_adapter.spec.ts` — happy path with a mock adapter program, reject if adapter not in registry (`AdapterNotApproved`), reject if non-PDA-owned writable token account, reject if CPI target is a forbidden program ID, daily-cap exhaustion still enforced, frequency cap still enforced
- [ ] 6.3 Add `programs/mock-adapter` test program crate — a tiny Anchor adapter that echoes back `call_data` for use in execute_via_adapter tests
- [ ] 6.4 Delete `tests/execute_swap.spec.ts` and `tests/execute_lending_op.spec.ts`
- [ ] 6.5 Update `tests/smoke.ts` — replace swap/lending paths with one `add_adapter` → `execute_via_adapter` end-to-end pass against the mock adapter

## 7. SDK (`@enclz/sdk`)

- [ ] 7.1 Add `addAdapter`, `removeAdapter`, `executeViaAdapter` helper functions
- [ ] 7.2 Add `getAdapterRegistry(groupConfigPda)` reader that decodes the PDA
- [ ] 7.3 Remove `swap`, `deposit`, `withdraw` helpers (downstream callers move to `executeViaAdapter` against the appropriate first-party adapter program ID)
- [ ] 7.4 Bump SDK major version (breaking change)
- [ ] 7.5 Regenerate IDL via `anchor build`; re-export the regenerated types

## 8. First-party adapter programs (separate repos)

- [ ] 8.1 Scaffold `enclz/adapter-template` repo — reference Anchor program with the standard adapter entry-point + `constraints` parsing pattern + tests + README
- [ ] 8.2 Scaffold `enclz/enclz-jupiter-adapter` repo — port the Jupiter-v6 CPI logic from the deleted `execute_swap.rs` plus `constraints` parsing (allowed input/output mints)
- [ ] 8.3 Scaffold `enclz/enclz-kamino-adapter` repo — port Kamino CPI logic plus `constraints` parsing (allowed market PDAs)
- [ ] 8.4 Scaffold `enclz/enclz-save-adapter` repo — same shape as Kamino adapter for Save markets
- [ ] 8.5 Wire each adapter's CI to verify-build, run unit tests, and publish IDL on tag
- [ ] 8.6 Each adapter freezes upgrade authority after v0.1.0 release + audit

## 9. Devnet release

- [ ] 9.1 Generate fresh `target/deploy/enclz-keypair.json` for v1.0 program ID
- [ ] 9.2 Update `Anchor.toml` `[programs.*]` sections to the new program ID
- [ ] 9.3 Update `declare_id!` in `programs/enclz/src/lib.rs`
- [ ] 9.4 Run `./scripts/verify-devnet-onchain.sh` (or the local equivalent) against the new deployment
- [ ] 9.5 Deploy the three first-party adapter programs to devnet
- [ ] 9.6 Update `docs/RELEASE_MANIFEST.devnet.json` with v1.0 program ID + each adapter program ID
- [ ] 9.7 Bump program version to `v1.0.0` in `programs/enclz/Cargo.toml`

## 10. Build + verify

- [ ] 10.1 `cargo test --package enclz` — all unit tests pass
- [ ] 10.2 `anchor build` — clean compile, IDL regenerated
- [ ] 10.3 `cargo clippy --all-targets -- -D warnings` — no warnings
- [ ] 10.4 LiteSVM runtime tests pass (`cargo test --manifest-path tests/runtime/Cargo.toml`) if applicable to this repo
- [ ] 10.5 Localnet end-to-end (`scripts/localnet-e2e.py` equivalent) — provisions a group, adds the mock adapter, executes a transfer + an `execute_via_adapter` through it, asserts policy caps
- [ ] 10.6 `npm run lint` (workspace root) passes
