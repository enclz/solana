## 1. State struct

- [x] 1.1 Remove `approved_amount` and `amount_used` from `WhitelistEntry` struct in `programs/enclz/src/state/whitelist_entry.rs`

## 2. Instruction handlers

- [x] 2.1 Remove `approved_amount` parameter from `add_to_whitelist` (handler signature + `#[instruction]` attr in lib.rs), remove `require!(approved_amount > 0)`, remove field writes for both removed fields
- [x] 2.2 Remove `approved_amount` parameter from `renew_whitelist_entry` (handler signature + `#[instruction]` attr in lib.rs), remove `approved_amount >= amount_used` guard, keep only TTL update
- [x] 2.3 Remove `amount_used` projection, cap check, `should_void`, and post-transfer increment/close from `execute_transfer`; keep TTL expiry check for EXTERNAL entries
- [x] 2.4 Remove `approved_amount = 0` and `amount_used = 0` writes from `add_agent` handler
- [x] 2.5 Remove `approved_amount = 0` and `amount_used = 0` writes from `initialize_group` handler

## 3. Error enum

- [x] 3.1 Add tombstone comment on `WhitelistAmountExhausted` variant in `errors.rs` marking it as retired (never emitted)

## 4. Unit tests

- [x] 4.1 Update `init_space_whitelist_entry_matches_field_layout` to expect new size (106)
- [x] 4.2 Update `whitelist_entry_round_trip_through_init_space_buffer` to drop `approved_amount`/`amount_used` from the test struct
- [x] 4.3 Verify `error_variants_have_stable_codes` still passes (WhitelistAmountExhausted stays at discriminant 2)

## 5. Integration tests

- [x] 5.1 Update `tests/owner_instructions.spec.ts`: remove `approvedAmount` assertions and parameter from `addToWhitelist`/`renewWhitelistEntry` calls
- [x] 5.2 Update `tests/execute_transfer.spec.ts`: remove the auto-void exhaustion test, update external entry setup helpers to drop `approvedAmount`
- [x] 5.3 Update `tests/smoke.ts`: remove the 5-transfer exhaustion loop, replace with a single transfer + balance check
- [x] 5.4 Update `tests/execute_swap.spec.ts` and `tests/execute_lending_op.spec.ts`: remove `approvedAmount` from external entry setup if present (not needed — these use protocol entries)

## 6. Build and verify

- [x] 6.1 Run `cargo test --package enclz` — all unit tests pass (30/30)
- [x] 6.2 Run `anchor build` — compiles cleanly, IDL regenerated
- [x] 6.3 Integration tests pass against local validator (28 execute_transfer, 27 owner_instructions)
- [x] 6.4 Run `npm run lint` — no lint errors
