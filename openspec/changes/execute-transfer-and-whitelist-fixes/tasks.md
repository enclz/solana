## 1. Error enum expansion

- [x] 1.1 Append `RecipientInvalid` variant to `EnclzError` in `programs/enclz/src/errors.rs` (after `InvalidTokenAccount`)
- [x] 1.2 Append `InvalidEntryType` variant to `EnclzError` in `programs/enclz/src/errors.rs`
- [x] 1.3 Update `error_variants_have_stable_codes` test in `programs/enclz/src/lib.rs` to pin positions 14 (`RecipientInvalid`) and 15 (`InvalidEntryType`)

## 2. WhitelistEntry schema and target field

- [x] 2.1 Add `target: Pubkey` field to `WhitelistEntry` struct in `programs/enclz/src/state/whitelist_entry.rs` (after `label`)
- [x] 2.2 Update `add_to_whitelist` handler in `programs/enclz/src/instructions/add_to_whitelist.rs` to store `entry.target = _target_address`
- [x] 2.3 Update `add_agent` handler in `programs/enclz/src/instructions/add_agent.rs` to store `intra_group_entry.target = agent_wallet.key()`

## 3. InvalidEntryType error swap

- [x] 3.1 In `add_to_whitelist.rs`, replace `EnclzError::Unauthorized` with `EnclzError::InvalidEntryType` on line 42 (INTRA_GROUP reject)
- [x] 3.2 In `add_to_whitelist.rs`, replace `EnclzError::Unauthorized` with `EnclzError::InvalidEntryType` on line 49 (unknown entry_type reject)

## 4. execute_transfer refactor

- [x] 4.1 Rewrite `compute_fee` in `programs/enclz/src/util/fee.rs` to return `(total, fee)` where `total = amount + ceil(amount * 10 / 10000)` (additive fee)
- [x] 4.2 Update `compute_fee` unit tests in `util/fee.rs` for additive math — test amounts 300_000, 1_000_000, 99, 0
- [x] 4.3 Add `recipient_wallet: UncheckedAccount` to `ExecuteTransferAccountConstraints` with `constraint` attributes for `RecipientInvalid` (both protocol_fee_wallet and agent_wallet.key() checks)
- [x] 4.4 Add `mint: Account<'info, Mint>` to the execute_transfer accounts struct with `constraint = mint.key() == agent_wallet.mint`
- [x] 4.5 Add `associated_token_program: Program<'info, AssociatedToken>` to the execute_transfer accounts struct
- [x] 4.6 Change `to_token_account` constraints from `mut` + `mint` to `init_if_needed` with ATA constraints and `payer = backend_operator`
- [x] 4.7 Change `whitelist_entry` PDA seed from `to_token_account.owner.as_ref()` to `recipient_wallet.key().as_ref()`
- [x] 4.8 Update CPI transfers in `execute_transfer.rs` handler: transfer `amount` to recipient, `fee` to fee wallet
- [x] 4.9 Verify `spent_today` counter still uses request `amount` (not `total`)

## 5. Documentation and spec fixes

- [x] 5.1 Fix `openspec/specs/transfer-execution/spec.md` Scenario "Recipient not whitelisted" to state `AccountNotInitialized` (3012) not `WhitelistViolation`
- [x] 5.2 Fix `openspec/specs/transfer-execution/spec.md` fee requirement and account constraints for all changed behavior
- [x] 5.3 Fix `openspec/specs/transfer-execution/spec.md` Purpose header (currently "TBD - created by archiving...")
- [x] 5.4 Update `docs/SPECIFICATION.md` enforcement list, fee math, to_token_account description, and whitelist target field

## 6. Build, test, deploy

- [x] 6.1 Run `cargo test --package enclz` — all unit tests pass (30/30)
- [x] 6.2 Run `anchor build` — program compiles cleanly
- [x] 6.3 Rust integration tests pass with new accounts and fee math (75/75)
- [x] 6.4 Regenerate SDK/IDL via `npm run build:sdk`
- [ ] 6.5 Deploy to devnet
