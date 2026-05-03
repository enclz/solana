## Context

Program logic is complete after the prior three changes. To hand off to the backend team and to prepare for an external audit, we need (a) a deterministic deploy path, (b) a smoke test that runs against real devnet to catch CPI / ATA / RPC issues missed by LiteSVM, and (c) CI gates that prevent regressions.

## Goals / Non-Goals

**Goals:**
- One command (`anchor run deploy:devnet`) takes a clean checkout to a deployed, working program.
- Smoke test exits 0 only when every step (provision → fund → transfer → auto-void → replay reject) works on devnet.
- CI runs on every push and blocks merge below quality bar.

**Non-Goals:**
- Mainnet deploy.
- External audit engagement (out of scope; this change makes the program audit-ready).
- Backend Node.js code, MCP server, or web app.
- Frontend wallet integration.

## Decisions

**Devnet RPC via QuickNode env var, not hardcoded URL.**
QuickNode tokens are per-developer; hardcoding leaks credentials. `Anchor.toml` references `${QUICKNODE_DEVNET_URL}`.

**Smoke test uses a fresh keypair per run** (generated + airdropped), not a long-lived devnet wallet.
Avoids state pollution between runs and lets CI run smoke in parallel without contention.

**Coverage tool: `cargo tarpaulin`.**
Considered `grcov` — tarpaulin is simpler in CI and adequate for Anchor. Threshold: 85% overall, 90% on `execute_transfer.rs` because it is the security boundary.

**`cargo deny` for license + supply chain, `cargo audit` for known CVEs.**
Both run in CI, both block on critical findings.

**`solana-security-txt` macro inline in `lib.rs`.**
Standard auditor onboarding artifact; trivial to add. The macro's `policy` field points at the repo's top-level `SECURITY.md`, which carries the disclosure / reporting policy.

**Mainnet deploy guard in `deploy.ts`.**
`migrations/deploy.ts` refuses `--mainnet` unless `--force-mainnet` is also passed. The intent is a friction-only safeguard against a single-sig key fat-fingering a mainnet upgrade before the upgrade authority is transferred to a Squads multisig. No separate doc — the constraint lives in the script and `SECURITY.md`.

## Risks / Trade-offs

- [Devnet RPC instability breaks CI smoke runs] → Smoke test runs only on `main` push, not every PR; PR-level CI uses local validator only.
- [Coverage gate too strict slows iteration] → 85% baseline chosen because instruction handlers are mostly straight-line code; revisit if it pushes contributors to write meaningless tests.

## Migration Plan

1. Land `add-devnet-deploy-pipeline` PR.
2. Run `npm run deploy:devnet` once locally; deploy script captures program ID, updates `declare_id!`, redeploys.
3. Smoke test passes.
4. Enable CI workflow on `main` branch.
5. Backend team integrates against the deployed program ID.

Rollback: program-ID rotation requires deploy with same upgrade authority. Devnet authority is the deploy keypair at `.solana/keys/devnet-deployer.json`; mainnet authority must be transferred to a Squads multisig before any user funds touch the program (rotation: `solana program set-upgrade-authority`).

## Open Questions

- Whether to set up a separate "staging" cluster (devnet-2) for backend integration vs. dev iteration → defer; one devnet program is fine for v1.
- Audit firm choice → out of scope for this change.
