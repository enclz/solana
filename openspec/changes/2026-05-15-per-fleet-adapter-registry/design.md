## Context

The Enclz core program currently enumerates every supported protocol as a dedicated `execute_*` instruction. As of v0.5.x the core ships three: `execute_transfer`, `execute_swap` (Jupiter), and `execute_lending_op` (Kamino/Save). Each new protocol the team wants to support is a fresh instruction with bespoke CPI plumbing, account validation, and tests.

Three pressures push against this model:

1. **Program-size and audit-surface growth.** BPF binaries have a ceiling and audits scale with surface area. A backlog of integrations (Drift, Phoenix, Marginfi, Meteora, Orca, NFT marketplaces, perps, oracles, vaults, …) means the core trends toward a monolith. Immutability — the only credible commitment that "Enclz will never push a malicious upgrade" — gets pushed further out the longer this continues.
2. **Trust model coherence.** Enclz's pitch is non-custodial: the group owner is the trust root, the chain is the enforcer, and Enclz the operator can be compromised without funds being drainable beyond policy ceilings. A globally-governed adapter registry (multisig, DAO, etc.) re-introduces Enclz as a trust dependency for protocol approval. A per-fleet, owner-controlled registry keeps the trust root where it already is.
3. **Competitive context.** QuantuLabs shipped `Agent-Vault` to devnet on 2026-05-14 with an `execute_cpi_checked` instruction that allows arbitrary CPI under structural post-checks (no Token/ATA/loader targets, custody verification, no SPL multisig satisfiable by the wallet PDA) — but **no per-target allowlist**. Enclz adds the pre-check gate on top of the same post-checks, which is strictly more conservative and sharpens the "policy-first" positioning.

## Goals / Non-Goals

**Goals:**
- Core program shrinks to: provisioning, whitelist, `execute_transfer`, `execute_via_adapter`, `add_adapter` / `remove_adapter`
- Group owner permissionlessly manages the adapter registry for their fleet
- The chain validates: adapter is on the owner's approved list (pre-check), amount caps respected, output ATAs are PDA-owned (post-check)
- `execute_swap` and `execute_lending_op` move out of the core to first-party adapter programs (Apache-2.0, out-of-tree)
- Owner authority surface unchanged: same wallet that signs `initialize_group` / `add_to_whitelist` / `add_agent` signs `add_adapter` / `remove_adapter`
- Per-adapter constraints (opaque `Vec<u8>` parsed by the adapter) let the owner narrow allowed inputs without the core understanding protocol semantics

**Non-Goals:**
- Global adapter approval / governance / multisig — explicitly off the table
- Cross-adapter composability (adapter calling adapter) — out of scope for v1.0; each `execute_via_adapter` is one hop
- Canonical-latest version pointers — exact-pin only (see Decision 1)
- Migration-window backwards compatibility with v0.x instructions — v1.0 is a clean break, served by a fresh program ID; v0.x deployment is deprecated, not patched

## Decisions

### Decision 1: Exact-pin adapter program IDs

The `GroupAdapterRegistry` stores literal `Pubkey` values for each approved adapter. There is no "canonical latest" pointer abstraction. Upgrading to a new adapter version means the owner calls `remove_adapter(old_id)` + `add_adapter(new_id, …)`.

Rationale:
- Trivially safe: an attacker controlling a "latest pointer" would gain backdoor authority to redirect every fleet pointing at it
- No extra state, no extra trust surface — keeps the v1.0 scope tight
- Owner-friendly upgrade flow is a webapp concern (notify owner of new adapter versions; one-click prompt to call remove+add)

### Decision 2: Fail-loud on removed adapters mid-flight

`execute_via_adapter` checks adapter membership at execution time. If the owner removed the adapter between the agent reserving an idempotency key and the operator executing the chain call, the on-chain instruction fails with `AdapterNotApproved` (new 6000-band variant).

Rationale:
- Trivial to implement — one check at execute-time, no snapshot logic, no per-intent state
- Owners get real-time control: pulling the adapter is immediately effective, no zombie in-flight calls
- Backend surfaces the error as a typed API response via `parseAnchorError` in `shared/anchor-errors.js`

### Decision 3: First-party adapters only for v1.0, primitive is permissionless

We ship `enclz-jupiter-adapter`, `enclz-kamino-adapter`, and `enclz-save-adapter` as first-party Apache-2.0 programs in v1.0. A reference `adapter-template` is also shipped (public repo) so the pattern is documented. The "How to write an adapter" guide and external-author support are deferred to v1.1 once the pattern has stabilized through real shipping.

The on-chain primitive itself is permissionless from day one — any owner can add any deployed program to their registry. We don't gate authorship, we just don't officially support external adapters until v1.1.

Rationale:
- Pre-release: don't promise a stable ABI we haven't proven yet
- First-party adapters cover the entire v0.x feature surface; no functional regression for migrating users
- Permissionless primitive means power users / sophisticated partners can ship their own adapters without waiting on us

### Decision 4: Empty default registry, curated off-chain catalog

A freshly-initialized group has no adapters. Only `execute_transfer` works until the owner explicitly calls `add_adapter`. The webapp ships a "Recommended Adapters" panel that surfaces the first-party adapter program IDs and lets the owner add them with one click. This is purely off-chain UX.

Rationale:
- "No silent defaults" matches the non-custodial principle — the owner sees and approves every program their fleet can call
- Curation lives off-chain in the webapp, not on chain — keeps the chain primitive purely permissionless

### Decision 5: New capability split — `adapter-management` and `adapter-execution`

Per existing capability naming (`whitelist-management` vs `transfer-execution`), the management surface and the execution surface are separate capabilities. `adapter-management` covers `add_adapter` / `remove_adapter` + the registry account layout requirements specific to mutation. `adapter-execution` covers `execute_via_adapter`. The shared registry account is documented in `program-state` (the canonical location for cross-capability data structures).

### Decision 6: Per-adapter `constraints: Vec<u8>` is opaque to the core

The adapter program is responsible for parsing `constraints` and validating the inbound call against it. The core stores the bytes verbatim and passes them through on every `execute_via_adapter` call. Each adapter defines its own `constraints` schema.

Rationale:
- Keeps the core protocol-agnostic — no new schema variants per adapter family
- Adapters encode their own whitelist semantics (e.g., the Kamino adapter parses "allowed-market list" out of its `constraints`; the perps adapter parses "max leverage"; etc.)
- Empty `constraints: Vec<u8>` is the "no per-adapter restriction" default

### Decision 7: New program ID for v1.0, no in-place upgrade from v0.x

v1.0 ships under a freshly-generated program keypair. v0.x continues to run for the migration window but is marked deprecated. Owners running the "Migrate Group" webapp flow re-initialize their group on the new program ID and add the recommended adapters via `add_adapter` calls signed from their wallet.

Rationale:
- Avoids the IDL-rotation pain of trying to upgrade a deployed program that's removing instructions
- Lets v1.0 freeze its upgrade authority shortly after audit (per the long-term immutability goal)
- The non-zero migration cost is acceptable pre-release

## Risks / Trade-offs

- **Risk: A malicious adapter program drains funds up to policy caps.** The chain validates the adapter is on the owner's approved list and that output ATAs are PDA-owned, but the adapter itself can call into arbitrary downstream programs. If the owner adds an adapter that proxies to a malicious target, the agent's daily-cap worth of funds is at risk. **Mitigation:** the same risk exists today for the whitelist (a malicious whitelist target could be an attacker-controlled wallet). Owners are responsible for due diligence on adapter program IDs they approve. The webapp curates a "Recommended" catalog. We add an "adapter authority status" indicator (frozen vs. upgradeable) to the webapp to help owners assess.
- **Risk: Adapter program upgradeable-authority compromise.** If a first-party adapter is still upgradeable and we lose key control, every fleet that registered that adapter ID is exposed. **Mitigation:** each adapter program freezes its upgrade authority shortly after release + audit, following the same long-term immutability path as the core.
- **Risk: Migration friction.** Owners who don't migrate lose swap/lending until they opt in. **Mitigation:** "Migrate Group" wizard in the webapp prompts one-tx adapter additions; in-product banner messaging; clear changelog. Pre-release, migration friction is acceptable.
- **Risk: Adapter discoverability is now an off-chain UX problem.** **Mitigation:** webapp panel with curated catalog; docs section explaining the registry; ecosystem partners can self-register their canonical adapter program IDs over time.
- **Trade-off: v0.x backend code (`server/lib/intents.js`) must be rewritten.** `executeSwap` / `executeLendingOp` callers either (a) remap to `executeViaAdapter` + an adapter program ID lookup, or (b) get deprecated and removed from the API. Decided at implementation time depending on how many client integrations exist.
- **Trade-off: SDK + MCP surface widens.** The MCP server gains an `executeAdapter` tool (or we keep `swap` / `deposit` / `withdraw` as named wrappers, each mapped to a canonical adapter program ID for that protocol). Trade is between (a) wider but lower-level tool surface, and (b) named/curated tools that hide the adapter abstraction. Decided alongside the backend deprecation path.
