## Why

Today the backend operator implicitly chooses which SPL mint moves on every token-touching instruction by selecting which token accounts to pass in. The program enforces only *parity* across legs (`from.mint == to.mint == fee.mint`), not an absolute pin. A compromised or buggy operator could therefore drain a non-policy mint out of any agent that happens to hold ATAs for multiple mints. The previous `add-execute-transfer` design explicitly deferred this hardening as future work; this change closes the gap by introducing a clear policy split:

- **Outbound paths** (`execute_transfer`) are pinned absolutely to a single "settlement mint" bound to the agent at `add_agent` time. Only the bound mint can ever leave agent custody.
- **Internal paths** (`execute_swap`) allow free rotation between any mints — the agent can swap USDC ↔ SOL ↔ memecoins ↔ … — but *every* swap output is constrained to remain in custody of the agent_wallet PDA. No third-party recipient is ever allowed.
- **Lending paths** (`execute_lending_op`) stay pinned to the bound mint (yield strategies are denominated in the settlement asset; cross-mint lending is out of scope).

The mental model: the bound mint defines what can leave; custody defines what can move. A compromised operator can churn the agent's holdings via swaps but cannot exfiltrate anything except the bound mint, and only through the rate-limited transfer path.

## What Changes

- Add a `mint: Pubkey` field to the `AgentWallet` PDA, written once at `add_agent` time and never mutated thereafter.
- `execute_transfer` requires all three token accounts (`from`, `to`, `protocol_fee`) to match `agent_wallet.mint`.
- `execute_swap` drops the input-mint constraint entirely; **adds** `to_token_account.owner == agent_wallet.key()` (custody pin) so swap output cannot land in a third-party wallet; drops daily and per-tx spend-limit checks (limits are mint-relative and meaningless across arbitrary mints — funds-stay-in-custody removes the theft threat anyway); **keeps** the hourly transaction cap; creates `protocol_fee_token_account` lazily via `init_if_needed` (rent paid by `backend_operator`) so the operator does not need to pre-provision a fee ATA for every possible input mint.
- `execute_lending_op` pins both `agent_token_account` and `protocol_fee_token_account` to `agent_wallet.mint`.
- `emergency_withdraw` drops the absolute mint pin in favor of mint-*parity* (`agent_token_account.mint == destination_token_account.mint`) so the owner can sweep any mint the agent has accumulated via swaps; the standalone `Mint` account is removed from the `Accounts` struct.
- **BREAKING**: `AgentWallet` account layout grows by 32 bytes; existing accounts on devnet are not migrated. Program version bumps `0.2.0 → 0.3.0` (greenfield, no production tenants — direct precedent: `add-group-name-to-group-config`).
- **BREAKING**: `emergencyWithdraw` IDL drops the `mint` account argument; SDK callers must update.
- **BREAKING**: `executeSwap` IDL drops `daily_limit` / `per_tx_limit` enforcement (the limits remain on the agent for transfer/lending, just not enforced in swap); the operator no longer needs a pre-existing fee ATA.
- Reuse the existing `EnclzError::InvalidMint` (code 6011); no new error variant — preserves the cross-system error-code contract pinned by `error_variants_have_stable_codes`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `program-state`: `AgentWallet` requirement gains a `mint: Pubkey` field; `INIT_SPACE` updates accordingly.
- `group-provisioning`: `add_agent` requirement captures the passed mint into `AgentWallet.mint`; `emergency_withdraw` requirement drops the standalone `Mint` account, requires mint parity between agent and destination ATAs (any mint, sweep-anything semantics).
- `transfer-execution`: `execute_transfer` requires `from_token_account.mint == agent_wallet.mint` (and the same for `to` / `protocol_fee`).
- `swap-execution`: `execute_swap` drops the input-mint constraint; adds `to_token_account.owner == agent_wallet.key()`; removes daily/per-tx spend-limit enforcement; lazily inits `protocol_fee_token_account`.
- `lending-execution`: `execute_lending_op` requires `agent_token_account.mint == agent_wallet.mint`.

## Impact

- **Onchain code**: `programs/enclz/src/state/agent_wallet.rs`, `instructions/{add_agent, execute_transfer, execute_swap, execute_lending_op, emergency_withdraw}.rs`, unit tests in `programs/enclz/src/lib.rs` (`init_space_agent_wallet_matches_field_layout`, `agent_wallet_round_trip_through_init_space_buffer`).
- **Versioning**: `programs/enclz/Cargo.toml` `version` and `lib.rs` `security_txt!.source_release` both bump to `0.3.0`. SDK regenerates downstream via `scripts/build-sdk.mjs`.
- **IDL surface**: `emergencyWithdraw` loses one account; `executeSwap` adds an `associated_token_program` dependency for `init_if_needed` (and may need an `input_mint` account to resolve the ATA seed). SDK consumers and the backend's call-site need updates.
- **Integration tests**: TypeScript e2e specs that call `emergencyWithdraw`; new wrong-mint rejection test in `tests/execute_transfer.spec.ts`; new "swap-output custody" test asserting `execute_swap` rejects a third-party `to_token_account`; new "free-mint swap" test asserting an agent bound to mint A can swap mint B → mint C as long as the output ATA is agent-PDA-owned.
- **Docs**: `docs/SPECIFICATION.md` `AgentWallet` field listing and the swap-flow section (submodule — push via SSH remote per CLAUDE.md).
- **Deploy**: devnet redeploy required; `.so` may need `solana program extend` if size grew.
