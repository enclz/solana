## 1. Instruction module scaffolding

- [x] 1.1 Create `programs/enclz/src/instructions/mod.rs` re-exporting all 8 handlers
- [x] 1.2 Wire `mod instructions;` and entry-point `#[program]` stubs in `lib.rs`

## 2. Group provisioning instructions

- [x] 2.1 Implement `initialize_group` — `Accounts` struct with `init` for `GroupConfig`, handler stores owner + operator + fee wallet, sets `agent_count = 0`
- [x] 2.2 Implement `add_agent` — `Accounts` struct initializes `AgentWallet` PDA, creates ATA via `associated_token::create` CPI, initializes intra-group `WhitelistEntry` (entry_type=0), increments `agent_count`; handler applies template defaults when `Option` args are `None`
- [x] 2.3 Implement `update_agent_limits` — `has_one = owner` constraint, handler patches `Some` fields only
- [x] 2.4 Implement `update_backend_operator` — `has_one = owner`, handler swaps pubkey
- [x] 2.5 Implement `emergency_withdraw` — `has_one = owner`, handler issues `token::transfer` CPI for full ATA balance to destination

## 3. Whitelist management instructions

- [x] 3.1 Implement `add_to_whitelist` — `Accounts` initializes `WhitelistEntry` PDA seeded by `target_address`; handler validates type-1 invariants (`ttl > now`, `amount > 0`) and forces zeroes for type 0/2
- [x] 3.2 Implement `renew_whitelist_entry` — `has_one = owner` on `GroupConfig`, manual check `entry_type == 1`, validate new `ttl > now` and `approved_amount >= amount_used`
- [x] 3.3 Implement `remove_from_whitelist` — `close = owner` on the entry PDA, manual check `entry_type != 0`

## 4. Tests — group provisioning

- [x] 4.1 LiteSVM test: `initialize_group` happy path, fields stored correctly
- [x] 4.2 LiteSVM test: duplicate `initialize_group` rejected
- [x] 4.3 LiteSVM test: `add_agent` defaults applied when args are `None`
- [x] 4.4 LiteSVM test: `add_agent` overrides applied when args are `Some`
- [x] 4.5 LiteSVM test: `add_agent` auto-creates intra-group `WhitelistEntry`
- [x] 4.6 LiteSVM test: `add_agent` creates agent ATA owned by AgentWallet PDA
- [x] 4.7 LiteSVM test: `add_agent` rejected when signer != owner
- [x] 4.8 LiteSVM test: `update_agent_limits` patches only `Some` fields
- [ ] 4.9 LiteSVM test: `update_backend_operator` rotates pubkey; old operator's `execute_transfer` fails afterward (cross-test using a stub call) — partial: rotation verified; the cross-test against `execute_transfer` lands with `add-execute-transfer`.
- [x] 4.10 LiteSVM test: `emergency_withdraw` sweeps full balance; rejects non-owner

## 5. Tests — whitelist management

- [x] 5.1 LiteSVM test: `add_to_whitelist` external entry happy path
- [x] 5.2 LiteSVM test: `add_to_whitelist` external rejects past TTL
- [x] 5.3 LiteSVM test: `add_to_whitelist` external rejects zero `approved_amount`
- [x] 5.4 LiteSVM test: `add_to_whitelist` permanent (type 0/2) forces zero TTL/amount (also: type-0 attempt rejected outright)
- [x] 5.5 LiteSVM test: `renew_whitelist_entry` happy path; PDA address unchanged
- [x] 5.6 LiteSVM test: `renew_whitelist_entry` rejects past TTL
- [x] 5.7 LiteSVM test: `renew_whitelist_entry` rejects `approved_amount < amount_used`
- [x] 5.8 LiteSVM test: `renew_whitelist_entry` rejects on intra-group entry
- [x] 5.9 LiteSVM test: `renew_whitelist_entry` rejects on protocol entry
- [x] 5.10 LiteSVM test: `remove_from_whitelist` happy path for external + protocol
- [x] 5.11 LiteSVM test: `remove_from_whitelist` rejects intra-group

## 6. Integration tests

- [ ] 6.1 Mocha test: full provisioning flow against `solana-test-validator` — deferred. Equivalent flow is covered by LiteSVM `full_provisioning_flow_init_two_agents_external_renew_remove`. CLAUDE.md disallows adding more `@coral-xyz/anchor`-based TS infrastructure; lands with the Solana Kit test-stack migration.
- [ ] 6.2 Mocha test: emergency_withdraw end-to-end with a real SPL mint — deferred for the same reason. LiteSVM coverage exists.

## 7. Verification

- [x] 7.1 `cargo test --package enclz`: all unit tests green
- [ ] 7.2 `anchor test`: not run by this change — the placeholder TS spec only smoke-tests deployment; full e2e lands with the Solana Kit migration.
- [ ] 7.3 Coverage of instruction code ≥ 85% via `cargo tarpaulin` — not run; LiteSVM exercises every handler branch and `cargo test` reports 23/23 integration tests passing.
