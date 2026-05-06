## Context

`GroupConfig` currently stores only operational keys and an `agent_count`. The backend already maps groups to display names in its own database, but every consumer (webapp, future dashboards, audit log exporters) duplicates that mapping or hits the backend just for a label. Backend tooling has asked for the label to live onchain so the IDL-decoded account is self-describing.

We are still pre-mainnet on devnet, with no production groups, so we can change the account layout without a migration.

## Goals / Non-Goals

**Goals:**
- Persist a stable, human-readable label per `GroupConfig`, set at init.
- Surface it through the IDL automatically so SDK consumers get it for free.
- Keep the change footprint small: one new field, one new instruction arg, no new errors or instructions.

**Non-Goals:**
- Renaming an existing group post-init (no `update_group_name`). Will be a follow-up if/when it's needed.
- Onchain UTF-8 / non-zero validation. Backend owns input hygiene.
- Any backwards-compatibility / realloc path for existing devnet `GroupConfig` accounts. Greenfield devnet, redeploy and re-init test fixtures.
- Webapp `package.json` bump or any client-side `decodeFixed32` helper — those live in a separate repo.

## Decisions

### Field layout: append after `agent_count`, fixed `[u8; 32]`

Append rather than insert. Rust struct field order is the Borsh wire order, so appending preserves the offsets of every existing field, which keeps any in-flight backend code that reads `owner` / `backend_operator` / `protocol_fee_wallet` / `agent_count` working unchanged after the redeploy.

`[u8; 32]` over `String`:
- Anchor's `InitSpace` derives a constant size for fixed arrays; `String` requires `#[max_len(N)]` and stores a 4-byte length prefix.
- Fixed-width buffers serialize identically across program upgrades — no surprise when an old client decodes a new layout.
- The label is a UI affordance, not a parser input; 32 bytes is enough for "Acme Trading Desk" with room. Backend pads short names with zeros and truncates anything longer.

### Argument order: `group_name` first

Putting `group_name` at the front of `initialize_group(...)` keeps the existing `(backend_operator, protocol_fee_wallet, dex_router)` triple visually grouped as "operational keys" and matches how the backend's REST handler already shapes the request body (`{ name, operator, fee_wallet, router }`). Argument-order changes are inherently breaking either way; this layout reads better at the call site.

### No realloc helper

We could ship a `realloc_group_config` instruction that grows existing accounts and zero-fills `group_name`, but no devnet group is currently load-bearing, and the testnet redeploy is going to reset the program upgrade authority anyway. Adding the realloc path would mean another instruction handler, another error case (`InvalidGroupConfigSize`?), and a one-time migration script — none of which earn their keep.

### Version bump 0.1.2 → 0.2.0

Account-layout breaks bump the minor (0.x). The version is single-sourced from `programs/enclz/Cargo.toml`; the IDL → SDK pipeline (`scripts/build-sdk.mjs`) syncs `sdk/package.json` automatically. The only piece that does **not** auto-sync is `security_txt!.source_release` in `programs/enclz/src/lib.rs:24`, which is compiled into the `.so` — that line bumps in the same commit.

### No new error variant

`EnclzError`'s order is a backend contract (Anchor numbers variants by enum position, offset 6000). Since the new arg has no validation path, nothing new can fail. Keep the enum frozen.

## Risks / Trade-offs

- **[Existing devnet groups become un-decodable]** → Acceptable per "greenfield devnet". Document in proposal; re-init test fixtures after redeploy.
- **[Backend/SDK drift]** — backend continues calling `initialize_group(backend_operator, protocol_fee_wallet, dex_router)` after redeploy → call fails with arg deserialization error. → Mitigation: cut SDK `0.2.0` only after redeploy; backend pins to `^0.2.0` in lockstep.
- **[Non-UTF-8 bytes break log/JSON serializers]** in clients that decode `group_name` as a string → Mitigation: this is a client concern; SDK exposes `number[]`. Webapp's `decodeFixed32` helper is responsible for lossy UTF-8 decode + zero-trim.
- **[`anchor keys sync` regression]** if anyone runs it during this work → CLAUDE.md already documents that it drops `[provider.devnet]` / `[provider.mainnet]` blocks and the unified program ID. Just don't run it.

## Migration Plan

1. Implement the program changes + bump version (single commit).
2. `anchor build` → IDL coverage check → `cargo test` → `npm run test:e2e` (mocha against `solana-test-validator`).
3. `npm run deploy:devnet` (uses the existing upgrade-authority key).
4. `node scripts/build-sdk.mjs` → publish `@enclz/sdk@0.2.0`.
5. Backend bumps SDK and starts sending the new arg.

**Rollback**: revert the program commit, redeploy `0.1.2`. SDK `0.1.x` is still on the registry and still works. Devnet test groups need re-init either way.

## Open Questions

None. All decisions confirmed in the prompt that triggered this change.
