## Context

`init-anchor-workspace` lands the state structs but no behavior. This change adds all instructions where the orchestrator is the signer — admin/provisioning surface only. `execute_transfer` (backend-signed, fund-moving) is split into a separate change because its logic is large enough to warrant focused review.

## Goals / Non-Goals

**Goals:**
- All 8 owner-signed instructions implemented + tested.
- Account constraints expressed via Anchor `#[derive(Accounts)]` + `has_one` / `seeds = [...]` so misuse fails at deserialization, not in handler logic.
- Per-instruction handler stays small — pure validation + state mutation, no fund transfers.
- Auto-add intra-group whitelist entry on `add_agent` so transfers between sibling agents work without orchestrator action.

**Non-Goals:**
- Token movement (only `emergency_withdraw` does this; spend-limited transfers belong to `add-execute-transfer`).
- Backend REST wiring.
- Anomaly webhooks (off-chain concern).

## Decisions

**`add_agent` creates the agent's USDC ATA in the same instruction via CPI to `associated_token::create`.**
Alternative: leave ATA creation to backend. Rejected — backend would need a second tx for every new agent and could fail mid-flight, leaving an agent PDA without an ATA.

**`add_agent` also auto-creates the intra-group whitelist entry for the new agent's pubkey.**
Why: spec mandates intra-group transfers always work; doing it in-instruction is atomic and avoids race conditions where the next agent registers before the orchestrator adds the entry. Implemented via `init` of a second account in the same `Accounts` struct.

**Owner enforcement via `has_one = owner` on `GroupConfig`** rather than manual signer checks.
Cuts boilerplate; Anchor verifies before handler runs. `initialize_group` is the only instruction where `owner` is set rather than checked.

**`Option<u64>` template-override args for `add_agent` + `update_agent_limits`.**
Lets backend pass `None` to mean "use spec default" without separate codepaths. Defaults pulled from `constants.rs`.

**`renew_whitelist_entry` keeps the same PDA**, only mutating `ttl_expires_at` + `approved_amount`. Closing + reopening would change the PDA address and break any backend cache referencing it.

**`remove_from_whitelist` rejects `entry_type == 0` (intra-group)** — those are structural to the group; removing them would silently break sibling transfers.

**`initialize_group` accepts a third arg `dex_router: Pubkey` and atomically creates a type-2 `WhitelistEntry` for that address.**
Why: SPECIFICATION.md requires the DEX swap router to be whitelisted as `entry_type = 2` at group init time. Doing it in the same instruction is atomic — no window where a group exists but the router isn't whitelisted, which would cause `execute_swap` (future change) to fail on first call.

**`add_to_whitelist` rejects `entry_type == 0`.**
Intra-group entries are structural artifacts created by `add_agent` only. Allowing manual creation lets orchestrators label arbitrary external addresses as permanent + uncapped, bypassing external entry TTL + amount enforcement entirely.

**Group-scoped (not agent-scoped) whitelist.**
`WhitelistEntry` seed = `["whitelist", group_pubkey, target_address]` — one entry covers the whole group, not one entry per agent. The target_address for intra-group entries is the sibling AgentWallet PDA pubkey. For external/protocol entries it is the recipient wallet or program pubkey. This means: if group G whitelists merchant M, ALL agents in G can send to M under that single entry's cap.

**`emergency_withdraw` bypasses limits + nonce.** Operator can't be involved (safety net for compromised operator scenario). Owner signs directly; transfers full ATA balance via `token::transfer` CPI.

## Risks / Trade-offs

- [Atomic intra-group whitelist creation could fail mid-init if account already exists] → Use Anchor `init_if_needed` cautiously, or pre-check; design to fail loudly so orchestrator retries.
- [`emergency_withdraw` is a footgun if owner key compromised] → Documented as defense-in-depth; future improvement is multi-sig (deferred per REQUIREMENTS.md).
- [Test surface explodes — 8 instructions × many reject branches] → Use a shared LiteSVM harness fixture (`fn setup() -> TestCtx`) to keep tests under 30 LoC each.
- [`Option` args can mask backend bugs (caller meant to set a value, sent None)] → Rely on REST-layer validation; on-chain treats `None` as explicit "use default".

## Migration Plan

Additive — no migration. Deploys after `init-anchor-workspace` via the same `anchor deploy` pipeline.

## Open Questions

- Whether `update_backend_operator` should require a confirmation window (defer for v1; add multisig later).
- Whether to emit Anchor events on each owner action so backend can subscribe instead of polling — defer; backend currently uses webhooks driven by RPC subscription.
