# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Orientation

Enclz is an Anchor program at `programs/enclz/` that enforces spend policy onchain for AI agent fleets. A separate backend (not in this repo) calls `execute_transfer` and translates Anchor errors to REST. The codebase is greenfield and driven by OpenSpec change proposals — see `README.md` for setup, deploy, and the project tour.

`docs/SPECIFICATION.md` is the source of truth for PDAs, seeds, account fields, sizing, and error names. `openspec/changes/init-anchor-workspace/specs/program-state/spec.md` formalizes those into requirements with WHEN/THEN scenarios. **If code disagrees with the spec, fix the code, not the spec** — and if `tasks.md` disagrees with `design.md` or `specs/`, fix `tasks.md` (precedent: commit `30e6585`).

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

Status of the five existing changes:

| Change | State |
|---|---|
| `init-anchor-workspace` | Mostly done — items 1.8 / 4.6 / 5.1 / 5.3 (anchor build/test from clean checkout) still pending |
| `add-owner-instructions` | Not started — `initialize_group`, `add_agent`, whitelist mgmt instruction handlers |
| `add-execute-transfer` | Not started — the core enforcement instruction |
| `add-execute-swap-and-yield` | Not started — Jupiter swap + lending CPIs |
| `add-devnet-deploy-pipeline` | Not started — CI, hardening, devnet deploy |

`openspec/changes/archive/` is for completed changes; once a change is fully implemented and shipped, archive it there.

### Local vs. cloud development environment

Same `Anchor.toml` paths, two materialization paths for the deployer keypair:

| Concern | Local | Claude Code on the web |
|---|---|---|
| RPC URLs | `.env` loaded per-command via `dotenv-cli` | Set as cloud env vars; auto-injected into the process |
| Deployer keypair | File at `.solana/keys/<cluster>-deployer.json` (gitignored) | `SOLANA_<CLUSTER>_DEPLOYER_KEYPAIR` env var → materialized into the same path by `.solana/init.sh` |
| Toolchain (Anchor, Solana CLI) | Manual install | Setup script in cloud env config (toolchain only — keypair belongs to the SessionStart hook so env-var rotation works without cache invalidation) |

`.solana/init.sh` is wired as a SessionStart hook in `.claude/settings.json`. It runs in both environments but short-circuits via `CLAUDE_CODE_REMOTE` when local — so locally, the file you generated with `solana-keygen new` is authoritative.

## Conventions

The `solana-anchor-claude-skill` is pinned via `skills-lock.json` and applies whenever you touch Anchor / Rust / TypeScript here. Read it for the full ruleset; the rules that bite hardest:

- **No yarn, npm only.** (Already cleaned out of `Anchor.toml`.)
- **No `@coral-xyz/anchor` for new TypeScript.** Use Solana Kit (`@solana/kit` + Kite). Existing `package.json` deps are tech debt from `anchor init`; don't add more.
- **No Coral XYZ or Solana Labs docs.** Anchor is at `https://github.com/solana-foundation/anchor`; Solana CLI docs at `https://docs.anza.xyz/`.
- **Anchor 1.0+ idioms.** `context.bumps.foo` (not `.get("foo").unwrap()`). `space = Foo::DISCRIMINATOR.len() + Foo::INIT_SPACE`.
- **Save `bump: u8` on every PDA struct.**
- **Terminology:** `onchain` / `offchain` (no hyphen), "program" not "smart contract", "Token Extensions Program" not "Token 2022".

The skill's "Do the whole thing" / "no placeholder tests" rules are also load-bearing — don't write `tests/foo.spec.ts` that just asserts the program exists; integration tests must initialize accounts, send transactions, verify state changes.

## Toolchain notes

- **Anchor 1.0.1's `anchor keys sync` rewrites `Anchor.toml`** and drops the per-cluster `[provider.devnet]` / `[provider.mainnet]` blocks (and the `[registry]` section). The blocks still parse correctly when restored — the normalizer just doesn't preserve them. If you ever run `keys sync` again, expect to re-add them and the unified program ID across `[programs.devnet]` / `[programs.mainnet]` (Anchor only updates `[programs.localnet]`).
- **`cargo-build-sbf` requires rustup** to manage the platform-tools toolchain. The Arch `solana` package ships only the runtime binaries — full toolchain comes from Anza's official installer (`https://release.anza.xyz/stable/install`), which lives in `~/.local/share/solana/install/active_release/bin`. Without rustup, you'll see `Failed to execute rustup: No such file or directory`.
- **TypeScript test stack is mocha + chai + ts-mocha + `@coral-xyz/anchor` 0.30.1.** Skill prescribes `node:test` + `tsx` + Solana Kit. The current stack works against an Anchor 1.0.1 program (verified), so migration is non-urgent — but don't add new TS infrastructure on top of `@coral-xyz/anchor`; pivot to Solana Kit when adding real instruction tests.

## Git conventions

Conventional commits: `feat(scope): ...`, `chore: ...`, `docs(scope): ...`. Match what `git log --oneline` shows. **Do not add `Co-Authored-By: Claude`** (per skill).
