## Context

`anchor build` already produces `target/idl/enclz.json` and `target/types/enclz.ts`, both gitignored (whole `target/` is ignored — see `.gitignore:1`). Tests already consume the generated TS via `import { Enclz } from "../target/types/enclz"` (see `tests/owner_instructions.spec.ts:20`, `tests/execute_transfer.spec.ts:20`), so the type contract is already exercised. What is missing is a way for code outside this repo to depend on the same artifacts.

Two audiences need this:

1. **Enclz backend (separate repo)** — calls `execute_transfer` and translates Anchor errors to REST. Needs `Program<Enclz>` typing and a stable `IDL` import.
2. **Public — security researchers, auditors, advanced integrators** — want to call the program directly without running `anchor build` themselves.

A third audience does **not** need this: AI agents using the Agent REST API + MCP server (the marketed primary user). The "no SDK required" positioning in `docs/MARKETING.md` and `docs/REQUIREMENTS.md:229` is about that path. The package shipped here is explicitly **program-level bindings**, not an agent SDK.

Anchor 1.0's generated `target/types/enclz.ts` exports only `type Enclz = {...}` — no `IDL` const (this changed from 0.30). So the SDK adds the JSON-as-IDL re-export itself.

## Goals / Non-Goals

**Goals:**
- One command (`npm run build:sdk`) takes a clean checkout to a `sdk/dist/` ready for `npm publish`.
- Two-line consumer setup: `npm install @enclz/sdk @coral-xyz/anchor @solana/web3.js` → `new Program<Enclz>(IDL, provider)`.
- `sdk/package.json` `version` is single-sourced from `programs/enclz/Cargo.toml` (via IDL `metadata.version`) so the published SDK version is always identifiable as the program version that produced it.
- `npm run idl:upgrade:<cluster>` puts the same IDL on-chain so `Program.fetchIdl(programId, provider)` works without the package.
- Public positioning explicit enough that a casual reader does not perceive `@enclz/sdk` as contradicting the "no SDK required" marketing claim.

**Non-Goals:**
- CI for releases — manual `npm publish` only. (Future change once cadence justifies it.)
- Migrating tests from `target/types/enclz.ts` to `@enclz/sdk` — tests stay on the direct path; the SDK is for external consumers.
- Replacing `idl/enclz.json` from `add-devnet-deploy-pipeline` — that change owns the committed IDL artifact in the program repo; this change owns distribution.
- Error-code map (`idl/error-map.json`) — also owned by `add-devnet-deploy-pipeline` (task 5.6 there).
- Bundling, minification, or CommonJS-vs-ESM dual builds — single CommonJS build matching root `tsconfig.json` is enough.

## Decisions

**Sub-package at `sdk/`, root stays private.**
Considered: flipping the root `package.json` to public and adding a `files` allowlist. Rejected — root carries dev-tooling deps (mocha, ts-mocha, @solana/spl-token) that should not enter the consumer dependency graph. A dedicated `sdk/` directory is the standard pattern (compare `@solana/spl-token` packaging within the Solana monorepo). No npm workspaces — keeps tooling simple; the root and SDK are treated as two co-located packages.

**Build copies generated artifacts into `sdk/src/`, then `tsc`.**
Considered: TS path mapping or `composite` projects pointing into `target/`. Rejected — published `dist/` would still need standalone files, and `tsc --outDir` does not flatten `../../target/...` imports. The simpler path: `cp target/idl/enclz.json sdk/src/` + `cp target/types/enclz.ts sdk/src/` (both gitignored), then `tsc -p sdk`. The copies are transient build inputs.

**Version single-source: `Cargo.toml` → IDL `metadata.version` → `sdk/package.json`.**
The build script reads `sdk/src/enclz.json` after copy, compares `metadata.version` to `sdk/package.json` `version`, and rewrites the latter if drift is detected. This means a developer bumps the program version in `programs/enclz/Cargo.toml` exactly once; the IDL regenerates on `anchor build` and the SDK picks it up on `npm run build:sdk`. Avoids three-way version drift. Trade-off: SDK and program must release in lockstep; if the SDK ever needs an out-of-band patch (e.g., README typo), the patch ships under the next program version. Acceptable for v1 — the SDK has near-zero handwritten code.

**Re-export shape: `IDL`, `PROGRAM_ID`, `type Enclz`.**
No type cast on `IDL` — `Program<Enclz>` accepts the JSON-shaped IDL as-is in Anchor 1.0+, so casting to `Enclz` (which has stricter literal types after generation) would force an `as unknown as Enclz` that adds nothing. Consumers who need the type use `import type { Enclz } from "@enclz/sdk"` separately.

**`@coral-xyz/anchor` as `peerDependency`, not `dependency`.**
The SDK ships only types + IDL JSON; it never imports the Anchor runtime. Consumers already need `@coral-xyz/anchor` to construct `Program`, so `peerDependencies: { "@coral-xyz/anchor": "^0.30.1" }` matches reality and avoids version conflicts. Range matches root `package.json` exactly.

**On-chain IDL: separate `init` and `upgrade` scripts per cluster (4 scripts total).**
Considered: a single `idl:publish` script that detects whether init or upgrade is needed. Rejected — too clever, and conditions are easy to check manually. `init` runs once after first deploy on each cluster; `upgrade` runs after every subsequent program upgrade. The unified program ID `67i3uY4gZaidynKa8XbNW569qACSVCebwKnLpNYVtWjj` is hardcoded in all four scripts (matches `Anchor.toml`). If the program ID ever changes, scripts and `Anchor.toml` change together.

**Manual `npm publish` for v1.**
Considered: GitHub Actions workflow on tag push (NPM_TOKEN secret + `actions/setup-node`). Rejected for now — release cadence is unknown, and a workflow file is dead weight until a tag actually lands. A future change can add it once the release pattern is real.

**Marketing positioning baked into the SDK README.**
The first paragraph of `sdk/README.md` says, in plain language, that this package is for direct on-chain callers and that AI agents should use the Agent REST API + MCP server. This is enforceable: the spec includes a scenario asserting the README mentions the agent-path alternative.

## Risks / Trade-offs

- [SDK version drift if developer publishes without `npm run build:sdk`] → `publish:sdk` chains both: `npm run build:sdk && cd sdk && npm publish --access public`. Direct `cd sdk && npm publish` is technically possible but discouraged in the README.
- [`@enclz` npm org not yet claimed] → Verified before first publish; if unavailable, fall back to `enclz-sdk` unscoped. This is a one-time setup gate, not a recurring risk.
- ["No SDK required" marketing claim perceived as contradicted by package's existence] → Mitigated by docs updates (`docs/REQUIREMENTS.md` adds "Program Integration Resources" section; `docs/MARKETING.md` footnotes the competitive table) and SDK README positioning. Spec includes a scenario that verifies these documents reference the program-vs-agent split.
- [Lockstep version coupling forces program version bumps for SDK-only fixes] → Accepted for v1. SDK has effectively no handwritten code; if this ever becomes painful, a future change can decouple by adding a `metadata.sdk_version` field to the IDL or moving to independent semver.
- [`anchor idl init/upgrade` requires upgrade authority signature] → Documented in `sdk/README.md` and in the relevant npm script's name (`idl:upgrade:devnet` is read as "I am the upgrade authority on devnet"). Backend operators without authority cannot run these scripts; that is correct behavior.
- [Submodule docs commits land out of order with this repo's changes] → `docs/` updates are a separate commit chain in `enclz/.github`. The recorded SHA bump in this repo waits for the docs PR to merge. tasks.md sequences this explicitly.

## Migration Plan

1. Land `add-idl-publishing` PR (this change) with `sdk/`, `scripts/build-sdk.mjs`, root scripts, root README distribution subsection. **No publish yet.**
2. Verify `npm publish --dry-run --access public` from `sdk/` shows the expected tarball contents.
3. Claim `@enclz` npm org if not already owned. Configure 2FA on the publishing account.
4. Land docs commits in `enclz/.github` (`REQUIREMENTS.md`, `MARKETING.md`). Bump submodule SHA in this repo via a follow-up commit.
5. First publish: `npm run publish:sdk` → version `0.1.0` (or whatever `Cargo.toml` reads at the time).
6. After devnet program deploy (gated by `add-devnet-deploy-pipeline`): `npm run idl:init:devnet` once, then `npm run idl:upgrade:devnet` on every subsequent upgrade.

Rollback: `npm unpublish` is restricted on npm (72-hour window for unscoped, never for scoped after the first download). If a bad SDK version ships, the rollback path is `npm publish` of the next patch version with the fix; consumers update via `npm install @enclz/sdk@latest`. There is no on-chain rollback for IDL — `anchor idl upgrade` overwrites; the only "rollback" is uploading the prior IDL again.

## Open Questions

- Whether `sdk/README.md` should include a code snippet for `Program.fetchIdl()` as a no-package alternative, or only mention the npm path → tentatively include both, since the on-chain IDL is one of the two channels this change ships.
- Whether to publish a parallel `@enclz/idl` package containing only the JSON (for non-TS consumers) → defer; on-chain IDL covers that audience.
- Whether to add a `prepublishOnly` hook in `sdk/package.json` that runs `node scripts/build-sdk.mjs` → likely yes; resolve during implementation.
