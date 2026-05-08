## 1. State

- [ ] 1.1 Add `pub mint: Pubkey,` field to `AgentWallet` in `programs/enclz/src/state/agent_wallet.rs`, placed immediately after `group`
- [ ] 1.2 Confirm `#[derive(InitSpace)]` regenerates `INIT_SPACE = 149` (32 group + 32 mint + 32 display_name + 8 + 8 + 1 + 8 + 1 + 8 + 8 + 8 + 1)

## 2. Provisioning

- [ ] 2.1 In `programs/enclz/src/instructions/add_agent.rs`, capture `agent_wallet.mint = context.accounts.mint.key();` inside `handle_add_agent` after `agent_wallet.group = ...`
- [ ] 2.2 Confirm the existing `associated_token::mint = mint` ATA constraint and the new field stay in sync (no separate enforcement needed at init — the `Mint` account is the source of truth)

## 3. Outbound paths — pin to bound mint

- [ ] 3.1 In `programs/enclz/src/instructions/execute_transfer.rs`, replace the cross-leg mint-parity constraints on `from_token_account` (`from.mint == to.mint`, `from.mint == fee.mint`) with three absolute pins: `from_token_account.mint == agent_wallet.mint`, `to_token_account.mint == agent_wallet.mint`, `protocol_fee_token_account.mint == agent_wallet.mint` — each with `@ EnclzError::InvalidMint`
- [ ] 3.2 In `programs/enclz/src/instructions/execute_lending_op.rs`, pin `agent_token_account.mint == agent_wallet.mint` and `protocol_fee_token_account.mint == agent_wallet.mint`; drop the existing parity line

## 4. Internal swaps — relax mint, lock custody

- [ ] 4.1 In `programs/enclz/src/instructions/execute_swap.rs`, **drop** the input-mint constraint (`from.mint == agent_wallet.mint`) and any cross-leg parity on `from`/`fee`; **drop** any constraint pinning the input or output mint to `agent_wallet.mint`
- [ ] 4.2 In `execute_swap.rs`, **add** `constraint = to_token_account.owner == agent_wallet.key() @ EnclzError::InvalidTokenAccount` on `to_token_account` — this is the load-bearing safety pin
- [ ] 4.3 In `execute_swap.rs`, add an `input_mint: Account<'info, Mint>` field so the fee-ATA seed can be expressed; constrain `from_token_account.mint == input_mint.key()` to keep the input mint coherent across the constraint expression
- [ ] 4.4 In `execute_swap.rs`, change `protocol_fee_token_account` to `init_if_needed` with `payer = backend_operator`, `associated_token::mint = input_mint`, `associated_token::authority = group_config.protocol_fee_wallet` (or read fee wallet via an additional account if Anchor cannot inline the field reference). Add `pub associated_token_program: Program<'info, AssociatedToken>` to the struct; import `anchor_spl::associated_token::AssociatedToken`
- [ ] 4.5 In `execute_swap.rs` handler, **remove** the `per_tx_limit` check, **remove** the `daily_limit` check, **remove** the daily reset call (`needs_daily_reset` / `last_spend_reset` / `spent_today`), and **remove** the `spent_today` increment. Keep the hourly reset and `tx_count_this_hour < hourly_tx_cap` check, and keep the `tx_count_this_hour` increment. Keep the nonce check, fee transfer, and Jupiter CPI

## 5. Owner sweep — mint-parity

- [ ] 5.1 In `programs/enclz/src/instructions/emergency_withdraw.rs`, **delete** the `pub mint: Account<'info, Mint>` field; remove the now-unused `Mint` import if no other constraint references it
- [ ] 5.2 On `agent_token_account`, replace `token::mint = mint` with just `token::authority = agent_wallet` (keep PDA ownership)
- [ ] 5.3 On `destination_token_account`, replace `token::mint = mint` with `constraint = destination_token_account.mint == agent_token_account.mint @ EnclzError::InvalidMint`

## 6. Versioning

- [ ] 6.1 Bump `programs/enclz/Cargo.toml` `version` from `0.2.0` to `0.3.0`
- [ ] 6.2 Bump `programs/enclz/src/lib.rs` `security_txt!.source_release` from `"v0.2.0"` to `"v0.3.0"` in the same commit (compiled into the `.so`; not auto-synced)

## 7. Rust unit tests

- [ ] 7.1 Update `init_space_agent_wallet_matches_field_layout` in `programs/enclz/src/lib.rs` to include `+ 32` for the new mint field; update the inline comment to enumerate `mint`
- [ ] 7.2 Update `agent_wallet_round_trip_through_init_space_buffer` in `programs/enclz/src/lib.rs` to populate `mint: Pubkey::new_unique()` in the struct literal and assert `decoded.mint == value.mint`
- [ ] 7.3 Verify `error_variants_have_stable_codes` still passes unchanged (no new variants — the contract guard confirms we kept that promise)

## 8. Integration tests

- [ ] 8.1 Update any helpers in `programs/enclz/tests/common/mod.rs` that construct or decode `AgentWallet` literals to include the new `mint` field
- [ ] 8.2 Add `execute_transfer_rejects_from_token_account_with_wrong_mint` (Rust or TS, matching the existing suite): provision an agent bound to mint A, mint a second SPL token B, build a `from_token_account` ATA owned by the agent_wallet PDA but with mint B, attempt `execute_transfer`, assert error code 6011 (`InvalidMint`)
- [ ] 8.3 Add `execute_swap_rejects_third_party_to_token_account`: provision an agent, attempt `execute_swap` with a `to_token_account` whose `owner != agent_wallet` PDA, assert account-constraint failure before any CPI
- [ ] 8.4 Add `execute_swap_allows_arbitrary_input_mint_into_pda_owned_output`: provision an agent bound to USDC, fund an SOL ATA owned by the agent_wallet PDA, mock-route a swap from SOL to a third mint M (also into a PDA-owned ATA), assert the swap succeeds and `agent_wallet.spent_today` is unchanged
- [ ] 8.5 Add `execute_swap_does_not_enforce_per_tx_or_daily_limit`: configure a tiny `per_tx_limit` and `daily_limit`, attempt a swap with `amount_in` far exceeding both, assert success (only `hourly_tx_cap` and the nonce gate the call)
- [ ] 8.6 Add `execute_swap_lazily_inits_fee_ata_for_novel_mint`: confirm the protocol fee wallet has no ATA for input mint M before the call, run the swap, assert the fee ATA exists post-call and `backend_operator` SOL balance decreased by approximately the rent-exempt minimum
- [ ] 8.7 Add `emergency_withdraw_sweeps_non_bound_mint`: provision an agent bound to USDC, plant a balance of mint M in the agent's PDA-owned ATA, call `emergency_withdraw` with both ATAs of mint M, assert the destination ATA receives the full balance and the agent ATA is empty
- [ ] 8.8 Add `emergency_withdraw_rejects_mint_mismatch`: pass an `agent_token_account` of mint A and a `destination_token_account` of mint B, assert `InvalidMint`
- [ ] 8.9 Update any TypeScript e2e call site invoking `emergencyWithdraw` to drop the `mint` account argument; update any `executeSwap` call site to add `associated_token_program` and (if applicable) `input_mint`

## 9. IDL + SDK regeneration

- [ ] 9.1 Run `anchor build` and confirm `target/idl/enclz.json` includes the new `AgentWallet.mint` field, that `emergencyWithdraw` no longer lists a `mint` account, and that `executeSwap` lists `associated_token_program` (and `input_mint` if added)
- [ ] 9.2 Run `node scripts/check-idl-coverage.mjs` — must report all 11 handlers present
- [ ] 9.3 Run `node scripts/build-sdk.mjs` — must regenerate `sdk/dist` and bump `sdk/package.json` to `0.3.0`

## 10. Verification

- [ ] 10.1 `cargo test --package enclz` passes (unit tests)
- [ ] 10.2 `npm run test:e2e` passes (mocha + solana-test-validator legacy), including all new tests in section 8
- [ ] 10.3 `npm run lint` passes

## 11. Spec sync (post-implementation)

- [ ] 11.1 Update `docs/SPECIFICATION.md` `AgentWallet` field listing (lines ~74–89) to include `mint: Pubkey` between `group` and `display_name`
- [ ] 11.2 Update `docs/SPECIFICATION.md` swap-flow section to describe the custody pin on `to_token_account.owner`, the dropped daily/per-tx limits on swaps, and the lazy fee-ATA initialization
- [ ] 11.3 Update `docs/SPECIFICATION.md` `emergency_withdraw` description to reflect mint parity (sweep any mint, owner-only, no standalone Mint account)
- [ ] 11.4 Commit in the `docs/` submodule and push via SSH remote per CLAUDE.md; bump the docs submodule pointer in this repo (`git add docs && git commit`)

## 12. Deploy

- [ ] 12.1 `npm run deploy:devnet` (use `solana program extend` first if `.so` outgrew the existing buffer)
- [ ] 12.2 Smoke-test the backend round-trip against the new SDK: (a) `addAgent` captures the mint, (b) `executeTransfer` rejects wrong-mint accounts, (c) `executeSwap` succeeds with a non-bound input mint as long as the output is PDA-owned, (d) `emergencyWithdraw` sweeps a non-bound mint
