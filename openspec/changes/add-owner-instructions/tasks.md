## 1. Instruction module scaffolding

- [ ] 1.1 Create `programs/enclz/src/instructions/mod.rs` re-exporting all 8 handlers
- [ ] 1.2 Wire `mod instructions;` and entry-point `#[program]` stubs in `lib.rs`

## 2. Group provisioning instructions

- [ ] 2.1 Implement `initialize_group` — `Accounts` struct with `init` for `GroupConfig`, handler stores owner + operator + fee wallet, sets `agent_count = 0`
- [ ] 2.2 Implement `add_agent` — `Accounts` struct initializes `AgentWallet` PDA, creates ATA via `associated_token::create` CPI, initializes intra-group `WhitelistEntry` (entry_type=0), increments `agent_count`; handler applies template defaults when `Option` args are `None`
- [ ] 2.3 Implement `update_agent_limits` — `has_one = owner` constraint, handler patches `Some` fields only
- [ ] 2.4 Implement `update_backend_operator` — `has_one = owner`, handler swaps pubkey
- [ ] 2.5 Implement `emergency_withdraw` — `has_one = owner`, handler issues `token::transfer` CPI for full ATA balance to destination

## 3. Whitelist management instructions

- [ ] 3.1 Implement `add_to_whitelist` — `Accounts` initializes `WhitelistEntry` PDA seeded by `target_address`; handler validates type-1 invariants (`ttl > now`, `amount > 0`) and forces zeroes for type 0/2
- [ ] 3.2 Implement `renew_whitelist_entry` — `has_one = owner` on `GroupConfig`, manual check `entry_type == 1`, validate new `ttl > now` and `approved_amount >= amount_used`
- [ ] 3.3 Implement `remove_from_whitelist` — `close = owner` on the entry PDA, manual check `entry_type != 0`

## 4. Tests — group provisioning

- [ ] 4.1 LiteSVM test: `initialize_group` happy path, fields stored correctly
- [ ] 4.2 LiteSVM test: duplicate `initialize_group` rejected
- [ ] 4.3 LiteSVM test: `add_agent` defaults applied when args are `None`
- [ ] 4.4 LiteSVM test: `add_agent` overrides applied when args are `Some`
- [ ] 4.5 LiteSVM test: `add_agent` auto-creates intra-group `WhitelistEntry`
- [ ] 4.6 LiteSVM test: `add_agent` creates agent ATA owned by AgentWallet PDA
- [ ] 4.7 LiteSVM test: `add_agent` rejected when signer != owner
- [ ] 4.8 LiteSVM test: `update_agent_limits` patches only `Some` fields
- [ ] 4.9 LiteSVM test: `update_backend_operator` rotates pubkey; old operator's `execute_transfer` fails afterward (cross-test using a stub call)
- [ ] 4.10 LiteSVM test: `emergency_withdraw` sweeps full balance; rejects non-owner

## 5. Tests — whitelist management

- [ ] 5.1 LiteSVM test: `add_to_whitelist` external entry happy path
- [ ] 5.2 LiteSVM test: `add_to_whitelist` external rejects past TTL
- [ ] 5.3 LiteSVM test: `add_to_whitelist` external rejects zero `approved_amount`
- [ ] 5.4 LiteSVM test: `add_to_whitelist` permanent (type 0/2) forces zero TTL/amount
- [ ] 5.5 LiteSVM test: `renew_whitelist_entry` happy path; PDA address unchanged
- [ ] 5.6 LiteSVM test: `renew_whitelist_entry` rejects past TTL
- [ ] 5.7 LiteSVM test: `renew_whitelist_entry` rejects `approved_amount < amount_used`
- [ ] 5.8 LiteSVM test: `renew_whitelist_entry` rejects on intra-group entry
- [ ] 5.9 LiteSVM test: `renew_whitelist_entry` rejects on protocol entry
- [ ] 5.10 LiteSVM test: `remove_from_whitelist` happy path for external + protocol
- [ ] 5.11 LiteSVM test: `remove_from_whitelist` rejects intra-group

## 6. Integration tests

- [ ] 6.1 Mocha test: full provisioning flow against `solana-test-validator` — init group → add 2 agents → add external whitelist entry → renew it → remove it
- [ ] 6.2 Mocha test: emergency_withdraw end-to-end with a real SPL mint

## 7. Verification

- [ ] 7.1 `cargo test --package enclz`: all unit tests green
- [ ] 7.2 `anchor test`: integration tests green
- [ ] 7.3 Coverage of instruction code ≥ 85% via `cargo tarpaulin`
