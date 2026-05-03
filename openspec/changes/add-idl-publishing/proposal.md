## Why

`anchor build` already emits `target/idl/enclz.json` and `target/types/enclz.ts`, but they are gitignored and have no distribution path. The Enclz backend (separate repo) currently has no way to depend on a versioned, typed Anchor client; external integrators have no way to consume the program at all. Shipping IDL + TS bindings via two complementary channels (npm + on-chain) gives the backend `Program<Enclz>` typing in two lines and gives explorers / auditors `anchor idl fetch` access without a package dependency.

This is orthogonal to `add-devnet-deploy-pipeline`, which commits `idl/enclz.json` to the repo and adds CI gates but explicitly excludes npm publishing and on-chain IDL — distribution is its own concern.

## What Changes

- New `sdk/` directory containing a publishable `@enclz/sdk` npm package that re-exports the Anchor IDL JSON, the `Enclz` TS type, and `PROGRAM_ID`.
- Build script `scripts/build-sdk.mjs` that copies generated artifacts from `target/` into `sdk/src/`, syncs `sdk/package.json` `version` from IDL `metadata.version` (which mirrors `programs/enclz/Cargo.toml`), and compiles to `sdk/dist/`.
- Root `package.json` scripts: `build:sdk`, `publish:sdk`, plus `idl:init:{devnet,mainnet}` and `idl:upgrade:{devnet,mainnet}` wrapping `anchor idl init/upgrade` for on-chain IDL publication.
- Root remains `"private": true`; only `sdk/package.json` publishes. Manual `npm publish` flow (no CI in this change — punted to a future change).
- Documentation positioning: `@enclz/sdk` is **program-level bindings** for direct on-chain callers (backend, advanced integrators, security researchers). It does **not** contradict the marketing claim "no SDK required" — that claim is about the agent → REST API path, not direct on-chain calls.
- `docs/` submodule updates (separate commits in `enclz/.github`):
  - `docs/REQUIREMENTS.md` — add a "Program Integration Resources" section alongside the existing "Agent Integration Resources" section.
  - `docs/MARKETING.md` — footnote on the "No SDK required" competitive table clarifying the scope of the claim.
  - Optional: `docs/SPECIFICATION.md` and `docs/profile/README.md` parallels.

## Capabilities

### New Capabilities
- `idl-publishing`: SDK package layout + exports, version-sync invariants between Cargo.toml/IDL/package.json, on-chain IDL upload scripts, public positioning relative to "no SDK required".

### Modified Capabilities
<!-- none — additive -->

## Impact

- Adds `sdk/{package.json,tsconfig.json,README.md,src/index.ts}`, `scripts/build-sdk.mjs`.
- Modifies root `package.json` (scripts only), `.gitignore` (ignore `sdk/dist/`, `sdk/src/enclz.{ts,json}`, `sdk/node_modules/`), root `README.md` (distribution subsection).
- Modifies `docs/` submodule files (`REQUIREMENTS.md`, `MARKETING.md`); bumps the recorded submodule SHA in this repo.
- Backend repo can `npm install @enclz/sdk @coral-xyz/anchor @solana/web3.js` and get `Program<Enclz>` typing.
- Public consumers can also fetch IDL via `Program.fetchIdl(programId, provider)` once `npm run idl:init:<cluster>` has been run after the corresponding deploy.
- Depends on `init-anchor-workspace` (already mostly done — generated artifacts exist). Does **not** depend on `add-devnet-deploy-pipeline`; the two changes can ship in either order.
- CI automation for SDK release is explicitly **out of scope** — manual `npm publish` until cadence justifies the workflow.
