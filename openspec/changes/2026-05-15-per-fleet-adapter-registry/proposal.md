## Why

Every new DeFi protocol Enclz wants to support today requires a dedicated `execute_*` instruction added to the core program (`execute_swap`, `execute_lending_op`, future `execute_perp_*`, `execute_nft_*`, etc.). This drives two problems:

1. **Core program grows monotonically.** Each protocol adds binary size, audit surface, and review burden. Long-term immutability of the core — the strongest possible security claim for the program — becomes harder to commit to with every protocol added.
2. **Trust shape conflicts with non-custodiality.** Either the core enumerates every protocol (slow, unbounded growth) or a generic `execute_arbitrary_cpi` is introduced (rejected on 2026-05-12 — see `project_enclz_no_generic_wallet_abstraction.md` memory). A globally-governed adapter registry would solve breadth but re-introduce trust in Enclz governance, defeating the non-custodial property.

Per-fleet, owner-controlled adapter registry resolves both: the core stays minimal, and protocol coverage scales through independently-deployed adapter programs whose program IDs are registered on a per-group basis by the group owner. The owner remains the trust root; the chain enforces the pre-check that any CPI target must be in the owner's approved list. Strictly more conservative than QuantuLabs' `Agent-Vault::execute_cpi_checked` (which has post-checks only).

## What Changes

- **NEW capability `adapter-management`:** introduces `GroupAdapterRegistry` PDA + `add_adapter` and `remove_adapter` instructions, owner-authority
- **NEW capability `adapter-execution`:** introduces `execute_via_adapter` instruction, operator-authority, validates adapter membership + amount caps + custody, CPIs to the adapter with agent-PDA signer seeds
- **BREAKING:** Remove `execute_swap` from core program (`programs/enclz/src/instructions/execute_swap.rs` deleted; entire `swap-execution` capability removed). Swap functionality moves to a first-party `enclz-jupiter-adapter` program shipped in a separate repository.
- **BREAKING:** Remove `execute_lending_op` from core program (`programs/enclz/src/instructions/execute_lending_op.rs` deleted; entire `lending-execution` capability removed). Lending functionality moves to first-party adapter programs (`enclz-kamino-adapter`, `enclz-save-adapter`, …) shipped in separate repositories.
- **BREAKING (default state):** A freshly-initialized group has an empty `GroupAdapterRegistry`. Until the owner calls `add_adapter`, only `execute_transfer` is available. Recommended adapters are surfaced as a curated catalog in the webapp orchestrator UI; the on-chain primitive itself is permissionless.
- Add `AdapterNotApproved` error variant to `ErrorCode` (next free 6000-band discriminant)

## Capabilities

### New Capabilities

- `adapter-management` — `GroupAdapterRegistry` PDA, `add_adapter`, `remove_adapter`
- `adapter-execution` — `execute_via_adapter` (the generic dispatch instruction)

### Modified Capabilities

- `program-state` — adds `GroupAdapterRegistry` account layout

### Removed Capabilities

- `swap-execution` — entire capability removed; replaced by adapter programs out-of-tree
- `lending-execution` — entire capability removed; replaced by adapter programs out-of-tree

## Impact

- **Program:** `programs/enclz/src/state/` (new `adapter_registry.rs`), `programs/enclz/src/instructions/` (delete `execute_swap.rs`, `execute_lending_op.rs`; add `add_adapter.rs`, `remove_adapter.rs`, `execute_via_adapter.rs`), `programs/enclz/src/lib.rs` (instruction registration changes), `programs/enclz/src/errors.rs` (add `AdapterNotApproved`)
- **Tests:** delete `programs/enclz/tests/execute_swap.rs`, `programs/enclz/tests/execute_lending_op.rs`; add `tests/adapter_management.rs`, `tests/execute_via_adapter.rs`; update `tests/common/mod.rs` helpers
- **Integration tests:** delete `tests/execute_swap.spec.ts`, `tests/execute_lending_op.spec.ts`; add `tests/adapter_management.spec.ts`, `tests/execute_via_adapter.spec.ts`; update `tests/smoke.ts`
- **SDK:** new `addAdapter`, `removeAdapter`, `executeViaAdapter` helpers in `@enclz/sdk`; `swap` / `deposit` / `withdraw` removed from the SDK (downstream callers move to `executeViaAdapter` against the appropriate adapter program ID)
- **Backend (webapp):** `server/lib/intents.js` loses `executeSwap` / `executeLendingOp`; gains `executeViaAdapter` that resolves adapter program IDs from the on-chain registry. The `/api/v1/swap`, `/api/v1/deposit`, `/api/v1/withdraw` routes either (a) are deprecated and removed, or (b) become thin wrappers that look up a canonical adapter program ID for the labeled protocol and call `executeViaAdapter`. Final shape decided in implementation.
- **MCP server (`@enclz/mcp`):** the `swap`, `deposit`, `withdraw` tools either follow the backend's deprecation path or collapse into a single `executeAdapter` tool. Decided alongside backend.
- **Docs:** `docs/SPECIFICATION.md` (`enclz/.github` submodule) needs an updated architecture section. `enclz/docs` site needs a new "Adapters" section covering the registry concept, the curated catalog, and how to register an adapter from the orchestrator UI. Per the user's standing rule, public docs are not updated in this change without explicit request.
- **OpenSpec:** new `program-state/spec.md` requirement; new top-level `openspec/specs/adapter-management/spec.md`, `openspec/specs/adapter-execution/spec.md` files materialized on archive; top-level `openspec/specs/swap-execution/` and `openspec/specs/lending-execution/` directories removed on archive.
- **First-party adapter repositories** (new, out-of-tree, Apache-2.0):
  - `enclz/enclz-jupiter-adapter`
  - `enclz/enclz-kamino-adapter`
  - `enclz/enclz-save-adapter`
  - `enclz/adapter-template` (reference template — public so external authors can mirror the pattern post-v1.0)
- **Devnet rotation:** v1.0 ships under a new program ID. Existing v0.x groups remain on the deprecated program until owners run the "Migrate Group" flow described in `design.md`.
