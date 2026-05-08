## Context

Today the program enforces *parity* across token accounts within a single instruction (`from.mint == to.mint == fee.mint` in `execute_transfer`; equivalent in `execute_swap` for input/fee, in `execute_lending_op`, and in `emergency_withdraw` via `token::mint = mint`). It does not enforce an *absolute* binding to any particular mint. The backend operator chooses the mint implicitly by selecting which token accounts to pass; if the operator is compromised or buggy and an agent has ATAs for multiple SPL mints (USDC + WSOL + a memecoin, for instance), the operator can spend the wrong one within the same per-tx and daily caps.

The previous `add-execute-transfer/design.md` recorded this gap explicitly — *"Mint consistency enforced; absolute USDC pin deferred"* — as future hardening work. The program is at version 0.2.0, has no production tenants, and the most recent breaking change (`add-group-name-to-group-config`) treated `GroupConfig` layout growth as a redeploy-without-migration. The same precedent applies here.

## Goals / Non-Goals

**Goals:**
- Bind each `AgentWallet` PDA to exactly one SPL "settlement mint" — the only mint that can ever leave agent custody.
- Allow free internal rotation between any mints via `execute_swap`, on the strict precondition that swap output stays in custody of the agent_wallet PDA.
- Reject any *outbound* path (`execute_transfer`, `execute_lending_op`) whose token accounts do not match the bound mint, before signer seeds are derived or counters rolled.
- Preserve the existing error-code contract (variant order in `EnclzError`) so the backend's REST translator does not need a coordinated update.
- Keep the change contained to onchain code + IDL + SDK regeneration; no migration tooling.

**Non-Goals:**
- Multi-mint *settlement* agents. An agent has exactly one outbound mint. If multi-settlement is required, the answer is "add another agent" — agents are cheap; binding is intentional.
- Migrating existing devnet `AgentWallet` accounts. Greenfield, redeploy.
- Restricting which mints `execute_swap` can route through. The custody pin on the output ATA is the safety constraint; the mint catalog is left open.
- Re-introducing daily/per-tx spend caps on swaps under a normalized unit (e.g., USD). Spend caps in the bound mint apply to outbound transfers and lending operations only; the hourly transaction cap covers swap rate-limiting.
- Adding a per-mint allowlist on `GroupConfig`. Out of scope.

## Decisions

**Field placement: append `mint: Pubkey` after `group` on `AgentWallet`.**
Rationale: `group` is the natural anchor point for "what this agent belongs to"; `mint` is the next-most-fundamental binding. Placement matters only to people reading raw account bytes — Anchor (de)serialization handles either order — and `agent_wallet.mint` reads cleanly in account-constraint expressions. Alternative (`mint` last, before `bump`) was considered; rejected because grouping the two `Pubkey` identity fields is more readable.

**Reuse `EnclzError::InvalidMint` (code 6011); do not append a new variant.**
Rationale: `lib.rs:365` (`error_variants_have_stable_codes`) pins all 14 variants by Anchor index because the backend matches errors by code (offset 6000) and translates to REST. Adding `WrongMint` would either (a) move existing codes — silently miscoding every backend error — or (b) sit at the end as a near-synonym of `InvalidMint`, splitting the operational mapping for no benefit. The semantic *"the mint passed does not match the mint pinned at init"* fits cleanly under `InvalidMint`'s existing meaning *"mint mismatch"*. Alternative (append `WrongMint`) was rejected.

**Drop `mint: Account<'info, Mint>` from `emergency_withdraw`'s `Accounts` struct.**
Once both ATAs are constrained to `token::mint = agent_wallet.mint`, the standalone `Mint` account is redundant. Removing it is a small IDL break that's free pre-mainnet and cleans up the SDK call site. Alternative (keep the `Mint` account and additionally constrain it to equal `agent_wallet.mint`) was rejected as cargo-cult — the constraint adds nothing the ATA mint check doesn't already enforce.

**No realloc / migration helper.**
Direct precedent: `add-group-name-to-group-config` (commit `ce6985f`) treated a 32-byte growth on `GroupConfig` as a redeploy. `AgentWallet` is in the same greenfield posture. A `realloc_agent_wallet` instruction would need its own handler, error path, integration tests, and an offchain orchestration script — none of which earn their keep at zero users. If users existed, a one-shot `migrate_agent_wallet(mint)` instruction signed by the group owner would be the right answer; that work is deferred until it's needed.

**Version bump in lockstep: Cargo `version`, `lib.rs` `source_release`.**
The SDK pipeline (`scripts/build-sdk.mjs`) propagates Cargo `version` into `sdk/package.json` automatically, but `security_txt.source_release` is compiled into the `.so` and is not auto-synced. Both must move to `0.3.0` in the same commit, before redeploy.

**Custody pin on `execute_swap` output (`to_token_account.owner == agent_wallet.key()`).**
This is the load-bearing constraint that lets us safely drop the input-mint pin on swaps. With the output ATA forced to be PDA-owned, a compromised operator who routes through Jupiter cannot send swap proceeds to a third-party wallet — the worst they can do is rotate the agent's holdings between mints they can't ultimately spend. The agent's outbound surface stays restricted to `execute_transfer`, which is still pinned to the bound mint. Alternative (whitelist allowed swap output mints) was rejected: the operational burden of curating a per-group mint catalog is high and adds nothing the custody pin doesn't already cover.

**Drop daily and per-tx spend-limit checks from `execute_swap`; keep `hourly_tx_cap`.**
The daily and per-tx limits were sized in the bound mint's units (e.g., USDC 6-decimal). With swap input now an arbitrary mint, "10 million" of mint X has no relationship to "10 million" of mint Y — the limit comparison is meaningless. Three options were considered:
- (a) Remove the checks. The threat the limits guarded against (theft) no longer applies, since funds stay in custody. ✓
- (b) Track per-mint daily counters. Adds N×16 bytes to `AgentWallet` per mint plus eviction logic; doesn't earn its keep.
- (c) Normalize via a price oracle. Adds an oracle dependency and a stale-price failure mode for an instruction that doesn't need it.

Picking (a). `hourly_tx_cap` still applies — that's a unit-free rate limit and protects against churn / market-impact abuse independent of mint identity. `spent_today` is *not* incremented on swaps under the new design.

**Lazy `protocol_fee_token_account` on swaps via `init_if_needed`.**
The fee comes out of the input mint, which now varies. Pre-provisioning a fee ATA for every conceivable mint is an operational burden the orchestrator shouldn't carry. `init_if_needed` makes the first swap of a novel mint pay rent (`backend_operator` is the payer) and subsequent swaps cheap. Implementation note: this requires adding `associated_token_program` to the `Accounts` struct, and likely a separate `input_mint: Account<'info, Mint>` so the ATA seed can be expressed in Anchor's constraint syntax (`associated_token::mint = input_mint`). Owner constraint stays `protocol_fee_wallet`.

**Relax `emergency_withdraw` to mint-parity rather than absolute pin.**
Direct consequence of the swap relaxation: the agent's PDA can now hold any mint. If `emergency_withdraw` only allowed the bound mint, every other mint the agent accumulated via swaps would be stranded. The fix is to constrain `agent_token_account.mint == destination_token_account.mint` — owner can sweep any mint, but both legs must agree (so a typo can't cross-mint). The standalone `Mint` account is still removed (we read mint identity off whichever token accounts are passed). Owner remains the only authorized signer.

## Risks / Trade-offs

- **Risk:** breaking SDK call site for `emergencyWithdraw` (drops the `mint` arg). → **Mitigation:** SDK regenerates from IDL; the only consumer is the project's own backend and the e2e tests. Update both in the same PR.
- **Risk:** an agent provisioned for the "wrong" mint (operator typo in `add_agent`) cannot be repurposed — only retired in favor of a new one. → **Mitigation:** that's intentional; binding is the point. Document in `proposal.md` and in the SDK's TSDoc for `addAgent`.
- **Risk:** swaps with no spend cap could churn the agent's holdings across mints, racking up fees and slippage. → **Mitigation:** `hourly_tx_cap` still throttles swap frequency. The orchestrator's offchain policy is the right layer for "don't swap more than $X/day" — the program enforces *what is reachable*, the orchestrator enforces *what is sensible*.
- **Risk:** an agent ends up holding a long tail of low-liquidity mints from runaway swaps, which `emergency_withdraw` must sweep one mint at a time. → **Mitigation:** acceptable. Each `emergency_withdraw` call sweeps one mint; the owner runs it per mint. A future change can add a multi-mint sweep instruction if it ever matters.
- **Risk:** `init_if_needed` on `protocol_fee_token_account` exposes a small rent-cost amplification: an attacker-operator could swap into novel mints repeatedly to bleed the operator's SOL on rent. → **Mitigation:** rent for an ATA is ~0.002 SOL; the operator's SOL balance is the orchestrator's responsibility to monitor. `hourly_tx_cap` (default 5) bounds the rate. If this becomes operationally painful, switch to a separate "register fee mint" admin instruction the owner runs once per mint.
- **Trade-off:** the `mint` ATA-vs-binding check is duplicated logically on every outbound instruction. → Could be hidden behind a Rust helper macro, but Anchor account constraints are already declarative — a helper would obscure more than it would save. Leave inline.

## Migration Plan

None. Steps to ship:

1. Implement onchain changes (state → instructions → tests).
2. Bump `programs/enclz/Cargo.toml` `version` and `programs/enclz/src/lib.rs` `source_release` to `0.3.0`.
3. `cargo test --package enclz` and `npm run test:e2e`.
4. `anchor build`, `node scripts/check-idl-coverage.mjs`, `node scripts/build-sdk.mjs`.
5. `npm run deploy:devnet` (use `solana program extend` if `.so` outgrew the existing buffer).
6. Update `docs/SPECIFICATION.md` `AgentWallet` field listing in the submodule; commit and push to `enclz/.github` via SSH; bump submodule pointer in this repo.

Rollback: redeploy the prior `0.2.0` artifact from `target/deploy/enclz.so` — the upgrade authority is unchanged.
