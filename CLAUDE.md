# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Orientation

Enclz is an Anchor program at `programs/enclz/` that enforces spend policy onchain for AI agent fleets. A separate backend (not in this repo) calls `execute_transfer` and translates Anchor errors to REST. The codebase is greenfield and driven by OpenSpec change proposals — see `README.md` for setup, deploy, and the project tour.

`docs/SPECIFICATION.md` is the source of truth for PDAs, seeds, account fields, sizing, and error names. `openspec/specs/program-state/spec.md` formalizes those into requirements with WHEN/THEN scenarios. **If code disagrees with the spec, fix the code, not the spec** — and if `tasks.md` disagrees with `design.md` or `specs/`, fix `tasks.md` (precedent: commit `30e6585`).

## Commands

```bash
# Rust unit tests (PDA derivation, INIT_SPACE round-trip, error code stability, constants)
cargo test --package enclz

# Single test
cargo test --package enclz tests::group_config_pda_matches_documented_seeds

# Build the program (requires anchor + solana CLI on host)
anchor build

# Mocha integration tests against solana-test-validator
npm run test:e2e                  # wraps `anchor test --validator legacy`
# Anchor 1.0 defaults to surfpool; the project uses solana-test-validator
# per design.md, so the --validator legacy flag is required.
# `anchor test` (without the flag) fails with "Failed to spawn `surfpool`".

# Deploy
npm run deploy:devnet             # dotenv-cli wraps the RPC URL env var
npm run deploy:mainnet

# Any other anchor command that consumes ${VAR} from Anchor.toml
npx dotenv -- anchor <command>
# Without `dotenv --`, ${QUICKNODE_DEVNET_RPC_URL} stays a literal string and the call fails.

# Lint
npm run lint
npm run lint:fix
```

## Architecture worth knowing

### Three PDAs in `programs/enclz/src/state/`

| Account | Seeds | Notable |
|---|---|---|
| `GroupConfig` | `["group", owner]` | Per-orchestrator policy admin |
| `AgentWallet` | `["wallet", group, agent_index_u8]` | Stores `bump: u8` to skip ~1500 CU per `execute_transfer` |
| `WhitelistEntry` | `["whitelist", group, target_address]` | `entry_type` 0/1/2 = intra-group / external (TTL+capped) / protocol; also stores `bump` |

All three derive `InitSpace`. Always size accounts as `8 + Foo::INIT_SPACE` — never hand-count, never make a `*_SIZE` const.

### `EnclzError` is a cross-system contract

`programs/enclz/src/errors.rs` variant *names* and *order* matter. The backend matches errors by name, and Anchor numbers them by enum position (offset 6000). `lib.rs` has a unit test pinning every variant's index — if you reorder or insert, that test breaks intentionally to remind you the backend will silently miscode.

`InvalidAddress` is intentionally absent (Pubkey ABI validation makes it unreachable). `InvalidTtl` is distinct from `InvalidAmount` so backend can distinguish TTL failures.

### OpenSpec drives the roadmap

`openspec/changes/<name>/` contains `proposal.md` (why + what), `design.md` (decisions + rationale), `tasks.md` (implementation checklist), and `specs/<capability>/spec.md` (requirements + scenarios). When implementing:

1. Read `proposal.md` + `design.md` + the capability spec(s) before writing code.
2. Work through `tasks.md`, marking items `[x]` as you complete them. Match the level of detail of existing entries.
3. If a task contradicts `design.md` or the spec, the spec wins — patch `tasks.md`.
4. Skills `openspec-explore`, `openspec-propose`, `openspec-apply-change`, `openspec-archive-change` exist but are optional helpers; the directory structure is the protocol.

No active OpenSpec changes are open right now — every proposal so far has been implemented and moved to `openspec/changes/archive/`. The capability specs in `openspec/specs/` are the current source of truth for behaviour. Start a new change directory under `openspec/changes/<name>/` when you take on the next piece of scope.

## Conventions

The `solana-anchor-claude-skill` is pinned via `skills-lock.json` and applies whenever you touch Anchor / Rust / TypeScript here. Read it for the full ruleset; the rules that bite hardest:

- **No yarn, npm only.** (Already cleaned out of `Anchor.toml`.)
- **No Coral XYZ or Solana Labs docs.** Anchor is at `https://github.com/solana-foundation/anchor`; Solana CLI docs at `https://docs.anza.xyz/`.
- **Anchor 1.0+ idioms.** `context.bumps.foo` (not `.get("foo").unwrap()`). `space = Foo::DISCRIMINATOR.len() + Foo::INIT_SPACE`.
- **Save `bump: u8` on every PDA struct.**
- **Terminology:** `onchain` / `offchain` (no hyphen), "program" not "smart contract", "Token Extensions Program" not "Token 2022".

**Skill rule that this project deliberately overrides:** the skill says "no `@coral-xyz/anchor` for new TypeScript, use Solana Kit." This project's `workspace-tooling` spec instead requires `@coral-xyz/anchor` + `mocha` + `chai` + `ts-node` for integration tests, and the init-anchor-workspace `design.md` picked that stack on purpose ("Mocha+test-validator catches real CPI + ATA wiring issues"). When the skill and the spec disagree, the spec wins — write new TS instruction tests with `@coral-xyz/anchor` + ts-mocha until/unless a future openspec change migrates the stack.

The skill's "Do the whole thing" / "no placeholder tests" rules are load-bearing — don't write `tests/foo.spec.ts` that just asserts the program exists; integration tests must initialize accounts, send transactions, verify state changes.

## Toolchain notes

- **Anchor 1.0.1's `anchor keys sync` rewrites `Anchor.toml`** and drops the per-cluster `[provider.devnet]` / `[provider.mainnet]` blocks (and the `[registry]` section). The blocks still parse correctly when restored — the normalizer just doesn't preserve them. If you ever run `keys sync` again, expect to re-add them and the unified program ID across `[programs.devnet]` / `[programs.mainnet]` (Anchor only updates `[programs.localnet]`).
- **`cargo-build-sbf` requires rustup** to manage the platform-tools toolchain. The Arch `solana` package ships only the runtime binaries — full toolchain comes from Anza's official installer (`https://release.anza.xyz/stable/install`), which lives in `~/.local/share/solana/install/active_release/bin`. Without rustup, you'll see `Failed to execute rustup: No such file or directory`.
- **TypeScript test stack is mocha + chai + ts-mocha + `@coral-xyz/anchor` 0.30.1.** Skill prescribes `node:test` + `tsx` + Solana Kit, but the project spec (`openspec/specs/workspace-tooling/spec.md` Requirement: Test framework wiring) mandates the Anchor + Mocha stack — see the Conventions section above. The 0.30.1 client interoperates fine with an Anchor 1.0.1 program (IDL is the contract).
- **`docs/` is a submodule of `enclz/.github` pinned to `branch = main`** (see `.gitmodules`). The recorded SHA is bumped per commit — `branch = main` only tells `--remote` which ref to advance to.
  - **Pushing branches requires SSH remote.** Clone defaults to HTTPS but the user's git is SSH-only; switch with `git -C docs remote set-url origin git@github.com:enclz/.github.git` before `git push`.
  - **After a docs PR merges**, do *not* run `git submodule update --remote --merge` while a feature branch is checked out in the submodule — it produces a local merge commit. Instead: `git -C docs checkout main && git -C docs pull --ff-only origin main && git -C docs branch -D <feature> && git -C docs push origin --delete <feature>`, then `git add docs && git commit` in the parent repo.
- **Local PATH for anchor + sbf tools.** Neither `~/.cargo/bin/anchor` nor `~/.local/share/solana/install/active_release/bin/` is on PATH in a fresh shell. Export both before any `anchor build` / `cargo build-sbf` / `npm run build:sdk` / `npm run deploy:*`: `export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"`.
- **`anchor deploy --provider.cluster devnet` ignores `[provider.devnet].wallet`.** It reads the default `[provider]` block and signs with `~/.config/solana/id.json`. The per-cluster wallet entries in `Anchor.toml` are aspirational — set `ANCHOR_WALLET=...` for the deploy step if you need a different signer. The actual upgrade authority is whichever key signed the first deploy.

## Versioning and IDL pipeline

- **Version is single-sourced from `programs/enclz/Cargo.toml` `version`.** Flow: Cargo → IDL `metadata.version` (via `anchor build`) → `sdk/package.json` (via `scripts/build-sdk.mjs`). Don't hand-edit `sdk/package.json` — it's regenerated.
- **`security_txt.source_release` in `programs/enclz/src/lib.rs:24` is NOT auto-synced.** It's compiled into the `.so`. Bump it in the same commit as the Cargo version, then redeploy.
- **`scripts/build-sdk.mjs` rebuilds when `target/idl/enclz.json` is older than any program source.** The earlier existence-only check shipped a stale IDL once (`execute_swap` + `execute_lending_op` missing for two commits).
- **`scripts/check-idl-coverage.mjs` runs in CI right after `anchor build`.** If it fails ("handlers absent from IDL"), 9× out of 10 you just need to re-run `anchor build` — Anchor 1.x does not actually choke on `<'info>` generics, despite folklore.

## Git conventions

Conventional commits: `feat(scope): ...`, `chore: ...`, `docs(scope): ...`. Match what `git log --oneline` shows. **Do not add `Co-Authored-By: Claude`** (per skill).
